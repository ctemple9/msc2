//! Terminal-native rendering for the first overview slice.
//!
//! The order mirrors the established MSC window: context header, controls
//! rail, server identity, section tabs, overview content, and console dock.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};

use super::app::{App, FocusTarget, SmallSurface};
use super::backups::BackupsState;
use super::console::ConsoleView;
use super::layout::{LayoutMode, ShellLayout};
use super::overview::TAB_NAMES;
use super::performance::{TrendMetric, format_bytes, format_memory_mb, format_metric};
use super::players::{PlayersState, profile_edition, profile_status};
use super::worlds::WorldsState;

pub fn render(frame: &mut Frame, app: &mut App) {
    app.poll_console();
    app.poll_activity();
    app.poll_sections();
    let layout = app.prepare_layout(frame.area());
    render_header(frame, layout.header, app, layout.mode);

    match layout.mode {
        LayoutMode::Wide => render_standard_shell(frame, &layout, app, true),
        LayoutMode::Medium => render_standard_shell(frame, &layout, app, false),
        LayoutMode::Small => render_small(frame, layout.content, app),
    }
    if app.confirmation().is_open() {
        render_confirmation(frame, layout.content, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, mode: LayoutMode) {
    let controls = match mode {
        LayoutMode::Wide => "[Tab] focus  [1-7] section  [a/x] lifecycle  [i] activity  [q] quit",
        LayoutMode::Medium => {
            "[r] rail  [c] console  [s] section  [a/x] lifecycle  [i] activity  [q] quit"
        }
        LayoutMode::Small => "[s] sections  [c] console  [i] activity  [?] help  [q] quit",
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
    if app.activity().is_open() {
        render_activity(frame, layout.content, app);
    } else {
        render_content(frame, layout.content, app);
    }
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
    if app.active_tab() == 1 {
        render_players(frame, area, app);
        return;
    }
    if app.active_tab() == 2 {
        if app.backups().open {
            render_backups(frame, area, app.backups());
        } else {
            render_worlds(frame, area, app, app.worlds());
        }
        return;
    }
    if app.active_tab() == 3 {
        render_performance(frame, area, app);
        return;
    }
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

fn render_worlds(frame: &mut Frame, area: Rect, app: &App, worlds: &WorldsState) {
    let title = if app.focus() == FocusTarget::Content {
        "› WORLDS"
    } else {
        "WORLDS"
    };
    let mut lines = Vec::new();
    if let Some(error) = &worlds.error {
        lines.push(Line::from(format!("World slots unavailable: {error}")));
    } else if !worlds.loaded {
        lines.push(Line::from("Loading world slots from the selected server…"));
    } else if worlds.detail_open {
        let Some(slot) = worlds.selected_slot() else {
            lines.push(Line::from("No world slot is selected."));
            render_box(frame, area, title, lines);
            return;
        };
        lines.push(Line::from("SELECTED WORLD SLOT"));
        lines.push(Line::from(format!(
            "{}  ·  {}{}",
            slot.name,
            if slot.is_active { "ACTIVE" } else { "SAVED" },
            if worlds
                .active_slot()
                .is_some_and(|active| active.id == slot.id)
            {
                " · live identity"
            } else {
                ""
            }
        )));
        lines.push(Line::from(format!("Slot ID: {}", slot.id)));
        lines.push(Line::from(format!("Created: {}", slot.created_at)));
        lines.push(Line::from(format!(
            "Archive: {}",
            slot.zip_size_bytes
                .map(format_archive_bytes)
                .unwrap_or_else(|| "not reported".to_string())
        )));
        lines.push(Line::from(format!(
            "Seed: {}",
            slot.world_seed.as_deref().unwrap_or("not reported")
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "[b] backups  [a] activate  [s] save current  [n] rename  [N] rename active",
        ));
        lines.push(Line::from(
            "[p] copy into  [l] replace active  [e] export  [v] convert  [R] repair",
        ));
        lines.push(Line::from("[d] delete  [esc] world list"));
    } else {
        let active = worlds
            .active_slot()
            .map(|slot| slot.name.as_str())
            .unwrap_or("none");
        lines.push(Line::from(format!("ACTIVE WORLD  {active}")));
        lines.push(Line::from(format!(
            "{} saved slots · server {}",
            worlds.slots().len(),
            worlds
                .response
                .as_ref()
                .map(|response| if response.server_running {
                    "RUNNING"
                } else {
                    "STOPPED"
                })
                .unwrap_or("unknown")
        )));
        lines.push(Line::from(""));
        if worlds.slots().is_empty() {
            lines.push(Line::from("No world slots are available."));
        } else {
            for (index, slot) in worlds.slots().iter().enumerate() {
                let marker = if index == worlds.selected_slot {
                    "›"
                } else {
                    " "
                };
                let state = if slot.is_active { "ACTIVE" } else { "SAVED" };
                lines.push(Line::from(format!(
                    "{marker} {}  {state}  {}",
                    slot.name, slot.id
                )));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            "[j/k] choose  [enter] detail  [c] create  [i] import  [r] reload",
        ));
    }
    if let Some((kind, value)) = &worlds.input {
        lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
    }
    if let Some(status) = &worlds.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    render_box(frame, area, title, lines);
}

fn render_backups(frame: &mut Frame, area: Rect, backups: &BackupsState) {
    let mut lines = Vec::new();
    if let Some(error) = &backups.error {
        lines.push(Line::from(format!("Backups unavailable: {error}")));
    } else if !backups.loaded {
        lines.push(Line::from(
            "Loading backups and schedule from the selected server…",
        ));
    } else {
        lines.push(Line::from("BACKUP CONTEXT FOR SELECTED WORLD"));
        let visible_backups = backups.visible_backups();
        lines.push(Line::from(format!("{} backups", visible_backups.len())));
        lines.push(Line::from(
            "Verification: per-backup verification is not exposed; the agent enforces restore/delete safety.",
        ));
        lines.push(Line::from(""));
        if visible_backups.is_empty() {
            lines.push(Line::from("No backups are available for this server."));
        } else {
            for (index, backup) in visible_backups.iter().enumerate() {
                let marker = if index == backups.selected_backup {
                    "›"
                } else {
                    " "
                };
                let kind = if backup.is_automatic {
                    "AUTO"
                } else {
                    "MANUAL"
                };
                let size = backup
                    .file_size
                    .map(format_archive_bytes)
                    .unwrap_or_else(|| "size unknown".to_string());
                lines.push(Line::from(format!(
                    "{marker} {}  {kind}  {}  {}",
                    backup.display_name, backup.trigger_reason, size
                )));
            }
        }
        lines.push(Line::from(""));
        if let Some(config) = &backups.config {
            lines.push(Line::from(format!(
                "SCHEDULE  {} · every {} min · retain {}",
                if config.auto_backup_enabled {
                    "ON"
                } else {
                    "OFF"
                },
                config.auto_backup_interval_minutes,
                config.auto_backup_max_count
            )));
        } else {
            lines.push(Line::from("SCHEDULE  unavailable"));
        }
        lines.push(Line::from(
            "[m] backup now  [r] restore  [d] delete  [c] edit schedule  [R] reload  [b] back",
        ));
    }
    if let Some(value) = &backups.input {
        lines.push(Line::from(format!(
            "Schedule enabled,true/false; interval minutes; retention count: {}_",
            value
        )));
    }
    if let Some(status) = &backups.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    render_box(frame, area, "› BACKUPS", lines);
}

fn render_box(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn format_archive_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KiB", bytes as f64 / 1024.0);
    }
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

fn render_players(frame: &mut Frame, area: Rect, app: &App) {
    let players = app.players();
    let title = if app.focus() == FocusTarget::Content {
        "› PLAYERS"
    } else {
        "PLAYERS"
    };
    let mut lines = Vec::new();
    if let Some(error) = &players.error {
        lines.push(Line::from(format!("Player data unavailable: {error}")));
    } else if !players.loaded {
        lines.push(Line::from("Loading player data from the selected server…"));
    } else if players.detail_open {
        render_player_detail(&mut lines, players);
    } else {
        lines.push(Line::from("ONLINE NOW"));
        lines.push(Line::from(players.online_summary()));
        if let Some(note) = &players.online.note {
            lines.push(Line::from(format!("Source: {}", player_note(note))));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("Roster  [j/k] choose profile  [enter] details"));
        if players.online.players.is_empty() {
            lines.push(Line::from("No players are online."));
        } else {
            for player in players.online.players.iter().take(12) {
                let identity = player
                    .uuid
                    .as_deref()
                    .map(|uuid| format!(" · {uuid}"))
                    .unwrap_or_default();
                lines.push(Line::from(format!("  {}{}", player.name, identity)));
            }
        }
        lines.push(Line::from(""));
        let hidden = players
            .profiles
            .iter()
            .filter(|profile| profile.is_hidden)
            .count();
        lines.push(Line::from(format!(
            "PLAYER DATA  {} profiles · sort: {} · hidden: {}",
            players.profiles.len().saturating_sub(hidden),
            players.sort.label(),
            if players.show_hidden {
                "shown"
            } else {
                "hidden"
            }
        )));
        if let Some((kind, value)) = &players.input {
            lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
        } else {
            lines.push(Line::from(format!(
                "Search: {}  [/] search  [s] sort  [H] hidden profiles",
                if players.profile_query.is_empty() {
                    "all"
                } else {
                    &players.profile_query
                }
            )));
            for (index, profile) in players.filtered_profiles().iter().enumerate().take(12) {
                let marker = if index == players.selected_profile {
                    "›"
                } else {
                    " "
                };
                lines.push(Line::from(format!(
                    "{marker} {}  {} · {}{}",
                    PlayersState::display_name(profile),
                    profile_status(profile),
                    profile_edition(profile),
                    if profile.is_op { " · operator" } else { "" }
                )));
            }
            if players.filtered_profiles().is_empty() {
                lines.push(Line::from("No stored profiles match the current search."));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "SESSION LOG  {} events · filter: {}",
            players.filtered_session_events().len(),
            if players.session_query.is_empty() {
                "all"
            } else {
                &players.session_query
            }
        )));
        for event in players.filtered_session_events().iter().rev().take(6) {
            lines.push(Line::from(format!(
                "  {}  {} {}",
                event.timestamp, event.player_name, event.event_type
            )));
        }
        if players.filtered_session_events().is_empty() {
            lines.push(Line::from("No session events match the current filter."));
        }
        if players.is_bedrock {
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "BEDROCK ALLOWLIST  {} entries  [a] add",
                players.allowlist.len()
            )));
            for name in players.allowlist.iter().take(6) {
                lines.push(Line::from(format!("  {name}")));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            "[f] session filter  [l] clear session history  [r] reload  [tab] move focus",
        ));
    }
    if let Some(status) = &players.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_player_detail(lines: &mut Vec<Line<'static>>, players: &PlayersState) {
    let Some(profile) = players.selected_profile() else {
        lines.push(Line::from("No profile is selected."));
        return;
    };
    lines.push(Line::from("PLAYER PROFILE"));
    lines.push(Line::from(format!(
        "{}  ·  {}  ·  {}",
        PlayersState::display_name(profile),
        profile_status(profile),
        profile_edition(profile)
    )));
    lines.push(Line::from(format!("Profile ID: {}", profile.id)));
    lines.push(Line::from(format!(
        "Skin lookup identity: {}",
        profile
            .skin_override_identifier
            .as_deref()
            .unwrap_or(&profile.image_identifier)
    )));
    if profile.is_op {
        lines.push(Line::from("Role: operator"));
    }
    if let Some(last_seen) = &profile.last_seen {
        lines.push(Line::from(format!("Last seen: {last_seen}")));
    }
    if let Some(stats) = &profile.stats {
        lines.push(Line::from(""));
        lines.push(Line::from("CURRENT DATA"));
        lines.push(Line::from(format!(
            "Health {:.1}/{:.1} · food {} · level {} · {}",
            stats.health,
            stats.max_health,
            stats.food_level,
            stats.xp_level,
            stats.game_mode_display
        )));
        lines.push(Line::from(format!(
            "Position x {:.0} y {:.0} z {:.0} · {}",
            stats.pos_x, stats.pos_y, stats.pos_z, stats.dimension_display
        )));
        lines.push(Line::from(format!(
            "Score: {} · XP total: {}",
            stats.score, stats.xp_total
        )));
    } else {
        lines.push(Line::from(
            "Current stats are not available for this profile.",
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "ACTIONS  [h] hide/show  [u] duplicate  [m] offline UUID  [t] custom UUID",
    ));
    lines.push(Line::from(
        "         [i] identify Bedrock  [o] skin override  [d] delete  [esc] back",
    ));
    if let Some((kind, value)) = &players.input {
        lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
    }
}

fn player_note(note: &str) -> &str {
    match note {
        "not_bedrock" => "online roster is only available for Bedrock servers",
        "no_active_server" => "no active server",
        other => other,
    }
}

fn render_performance(frame: &mut Frame, area: Rect, app: &App) {
    let performance = app.performance();
    let title = if app.focus() == FocusTarget::Content {
        "› PERFORMANCE"
    } else {
        "PERFORMANCE"
    };
    let mut lines = Vec::new();
    if let Some(error) = &performance.error {
        lines.push(Line::from(format!("Performance unavailable: {error}")));
    } else if !performance.loaded {
        lines.push(Line::from("Loading live performance data…"));
    } else if let Some(snapshot) = performance.current() {
        lines.push(Line::from("LIVE PERFORMANCE"));
        lines.push(Line::from(format!(
            "Source: {} · sample: {}",
            if performance.server_type.is_empty() {
                "unknown"
            } else {
                &performance.server_type
            },
            snapshot.ts
        )));
        lines.push(Line::from(""));
        if performance.server_type.eq_ignore_ascii_case("bedrock") {
            lines.push(Line::from(
                "TPS  Bedrock does not report TPS through this API.",
            ));
        } else {
            lines.push(Line::from(format!(
                "TPS       1m {}   5m {}   15m {}",
                format_metric(snapshot.tps_1m.as_ref().map(|m| m.value), "", 2),
                format_metric(snapshot.tps_5m.as_ref().map(|m| m.value), "", 2),
                format_metric(snapshot.tps_15m.as_ref().map(|m| m.value), "", 2)
            )));
            lines.push(Line::from(format!(
                "TPS trend {}  (old → new)",
                performance.trend(TrendMetric::Tps)
            )));
        }
        lines.push(Line::from(format!(
            "Players   {} currently online",
            snapshot
                .players_online
                .map_or_else(|| "—".to_string(), |value| value.to_string())
        )));
        lines.push(Line::from(format!(
            "CPU       {}  · trend {}",
            format_metric(snapshot.cpu_percent.as_ref().map(|m| m.value), "%", 1),
            performance.trend(TrendMetric::Cpu)
        )));
        lines.push(Line::from(format!(
            "Memory    {} / {}  · trend {}",
            format_memory_mb(snapshot.ram_used_mb.as_ref().map(|m| m.value)),
            format_memory_mb(snapshot.ram_max_mb.as_ref().map(|m| m.value)),
            performance.trend(TrendMetric::Memory)
        )));
        lines.push(Line::from(format!(
            "World     {}",
            snapshot
                .world_size_mb
                .as_ref()
                .map(|value| format_bytes(value.value * 1024.0 * 1024.0))
                .unwrap_or_else(|| "—".to_string())
        )));
        lines.push(Line::from(format!(
            "Uptime    {}",
            performance.uptime_label()
        )));
        lines.push(Line::from(format!(
            "Status    {} · {}",
            performance.status_label(),
            performance.status_detail()
        )));
        if let Some(note) = performance.runtime_note() {
            lines.push(Line::from(format!("Runtime note: {note}")));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            "[r] refresh  [tab] move focus  values are from /v1/performance",
        ));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_activity(frame: &mut Frame, area: Rect, app: &App) {
    let activity = app.activity();
    let mut lines = vec![Line::from(
        "Current session activity  [j/k] select  [x] cancel operation  [esc/i] close",
    )];
    for (index, operation) in activity.operations().enumerate() {
        let marker = if index == activity.selected_index() {
            "›"
        } else {
            " "
        };
        let progress = operation
            .progress
            .as_ref()
            .map(|value| format!(" {}/{}", value.current, value.total))
            .unwrap_or_default();
        lines.push(Line::from(format!(
            "{marker} OP  {}  {}{}  {}",
            operation.id,
            operation_state(operation.state),
            progress,
            operation.status_line.as_deref().unwrap_or("—")
        )));
    }
    let notification_offset = activity.operations().count();
    for (index, notification) in activity.notifications().enumerate() {
        let marker = if notification_offset + index == activity.selected_index() {
            "›"
        } else {
            " "
        };
        lines.push(Line::from(format!(
            "{marker} NOTE  {}  {}",
            notification.title, notification.body
        )));
    }
    if lines.len() == 1 {
        lines.push(Line::from(
            "No operations or notifications in this session yet.",
        ));
    }
    if let Some(status) = activity.status() {
        lines.push(Line::from(format!("Status: {status}")));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("› ACTIVITY"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_confirmation(frame: &mut Frame, area: Rect, app: &App) {
    let Some(request) = app.confirmation().request() else {
        return;
    };
    let text = format!(
        "Confirm request\n\nHost: {}\nServer: {}\nTarget: {}\n\n{}\n\n[enter/y] confirm  [esc/n] cancel",
        request.host, request.server, request.target, request.consequence
    );
    let width = area.width.saturating_sub(8).clamp(32, 84);
    let height = area.height.saturating_sub(8).clamp(8, 14);
    let modal = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("› CONFIRM"))
            .wrap(Wrap { trim: true }),
        modal,
    );
}

fn operation_state(state: msc_api::dto::OperationStateDto) -> &'static str {
    match state {
        msc_api::dto::OperationStateDto::Queued => "QUEUED",
        msc_api::dto::OperationStateDto::Running => "RUNNING",
        msc_api::dto::OperationStateDto::Succeeded => "SUCCEEDED",
        msc_api::dto::OperationStateDto::Failed => "FAILED",
        msc_api::dto::OperationStateDto::Cancelled => "CANCELLED",
    }
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
    if app.activity().is_open() {
        render_activity(frame, area, app);
        return;
    }
    match app.small_surface() {
        SmallSurface::Overview => {
            if app.active_tab() == 1 {
                render_players(frame, area, app);
                return;
            }
            if app.active_tab() == 2 {
                if app.backups().open {
                    render_backups(frame, area, app.backups());
                } else {
                    render_worlds(frame, area, app, app.worlds());
                }
                return;
            }
            if app.active_tab() == 3 {
                render_performance(frame, area, app);
                return;
            }
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
