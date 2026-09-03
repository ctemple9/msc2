//! Terminal-native rendering for the first overview slice.
//!
//! The order mirrors the established MSC window: context header, controls
//! rail, server identity, section tabs, overview content, and console dock.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};
use serde_json::Value;

use super::access::AccessSurface;
use super::agent::AgentSurface;
use super::app::{AdminSurface, App, FocusTarget, SmallSurface, SupportSurface};
use super::app_settings::AppSettingsSurface;
use super::backups::BackupsState;
use super::components::{ComponentSurface, ComponentsState};
use super::connections::ConnectionSurface;
use super::console::ConsoleView;
use super::files::FilesState;
use super::handbook::HandbookSurface;
use super::health::HealthState;
use super::layout::{LayoutMode, ShellLayout};
use super::manage_servers::{ManageServersState, ManageSurface};
use super::overview::TAB_NAMES;
use super::performance::{TrendMetric, format_bytes, format_memory_mb, format_metric};
use super::players::{PlayersState, profile_edition, profile_status};
use super::server_editor::{EditorSurface, ServerEditorState};
use super::settings::SettingsState;
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
    if app.support_surface().is_some() {
        render_support(frame, layout.content, app);
    }
    if app.confirmation().is_open() {
        render_confirmation(frame, layout.content, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, mode: LayoutMode) {
    let controls = match mode {
        LayoutMode::Wide => {
            "[Tab] focus  [1-7] section  [g] handbook  [A] agent  [,] settings  [?] help  [q] quit"
        }
        LayoutMode::Medium => {
            "[r] rail  [c] console  [s] section  [g] handbook  [A] agent  [,] settings  [q] quit"
        }
        LayoutMode::Small => {
            "[s] sections  [c] console  [g] handbook  [A] agent  [?] help  [q] quit"
        }
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
        "Host session\n{}\n\nSelected server\n{}\n\nLifecycle\n{}  [a] start  [x] stop\n\nManage Servers  [m]\nServices  [v]\nHow to connect  [h]\nMaintenance  [d]\nAccess  [u]\n\nHandbook [g]  Agent [A]\nMSC settings [,]  Help [?]\n\n{}",
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
    if let Some(editor) = app.editor() {
        render_server_editor(frame, area, editor);
        return;
    }
    if app.manage_servers().is_open() {
        render_manage_servers(frame, area, app.manage_servers(), app);
        return;
    }
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
    if app.active_tab() == 4 {
        render_components(frame, area, app.components());
        return;
    }
    if app.active_tab() == 5 {
        render_admin(frame, area, app);
        return;
    }
    if app.active_tab() == 6 {
        render_files(frame, area, app, app.files());
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

fn render_admin(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus() == FocusTarget::Content {
        "› SETTINGS / ACCESS"
    } else {
        "SETTINGS / ACCESS"
    };
    let mut lines = vec![Line::from(
        "[1] Settings  [2] Connections  [3] Health  [4] Access",
    )];
    lines.push(Line::from(
        "Agent-provided capability, permission, and help state.",
    ));
    lines.push(Line::from(""));
    match app.settings_surface() {
        AdminSurface::Settings => render_settings_lines(&mut lines, app.settings()),
        AdminSurface::Connections => render_connections_lines(&mut lines, app.connections()),
        AdminSurface::Health => render_health_lines(&mut lines, app.health()),
        AdminSurface::Access => render_access_lines(&mut lines, app.access()),
    }
    render_box(frame, area, title, lines);
}

fn render_files(frame: &mut Frame, area: Rect, app: &App, files: &FilesState) {
    let title = if app.focus() == FocusTarget::Content {
        "› FILES · READ ONLY"
    } else {
        "FILES · READ ONLY"
    };
    let mut lines = Vec::new();
    if let Some(error) = &files.error {
        lines.push(Line::from(format!("Files unavailable: {error}")));
    } else if !files.loaded {
        lines.push(Line::from("Loading the selected server's files…"));
    } else if files.detail_open {
        if let Some(preview) = &files.preview {
            let path = preview
                .path
                .as_deref()
                .map(display_file_path)
                .unwrap_or_else(|| "Server Root".to_string());
            lines.push(Line::from(format!(
                "Server: {}",
                preview.name.as_deref().unwrap_or("Selected file")
            )));
            lines.push(Line::from(format!("Path: {path}")));
            lines.push(Line::from(format!(
                "Size: {} · Encoding: {}{}",
                preview
                    .size_bytes
                    .map(format_file_size)
                    .unwrap_or_else(|| "unknown".to_string()),
                preview.encoding.as_deref().unwrap_or("unknown"),
                if preview.truncated.unwrap_or(false) {
                    " · preview truncated at the agent limit"
                } else {
                    ""
                }
            )));
            lines.push(Line::from(""));
            lines.extend(
                preview
                    .content
                    .as_deref()
                    .unwrap_or("The agent returned no preview content.")
                    .lines()
                    .map(|line| Line::from(line.to_string())),
            );
            lines.push(Line::from(""));
            lines.push(Line::from("[esc/b] file list  [y] report path"));
        } else {
            lines.push(Line::from("No file preview is selected."));
        }
    } else if let Some(response) = &files.response {
        lines.push(Line::from(format!(
            "Server: {}",
            response
                .server_name
                .as_deref()
                .unwrap_or(app.overview().selected_server_name())
        )));
        lines.push(Line::from(format!(
            "Path: {}{}",
            display_file_path(&response.path),
            response
                .parent_path
                .as_deref()
                .map(|parent| format!(" · parent {}", display_file_path(parent)))
                .unwrap_or_default()
        )));
        if let Some(note) = &response.note {
            lines.push(Line::from(format!("Agent note: {note}")));
        }
        lines.push(Line::from(""));
        let folders = response
            .items
            .iter()
            .filter(|item| item.is_directory)
            .collect::<Vec<_>>();
        let files_only = response
            .items
            .iter()
            .filter(|item| !item.is_directory)
            .collect::<Vec<_>>();
        let mut has_items = false;
        for (label, items) in [
            ("Folders", folders.as_slice()),
            ("Files", files_only.as_slice()),
        ] {
            if items.is_empty() {
                continue;
            }
            has_items = true;
            lines.push(Line::from(label));
            for item in items {
                let index = response
                    .items
                    .iter()
                    .position(|candidate| candidate.id == item.id)
                    .unwrap_or_default();
                let marker = if index == files.selected { "›" } else { " " };
                let kind = if item.is_directory {
                    "DIR"
                } else if item.is_previewable {
                    "PREVIEW"
                } else {
                    "FILE"
                };
                let metadata = if item.is_directory {
                    String::new()
                } else {
                    format!(
                        " · {}",
                        item.size_bytes
                            .map(format_file_size)
                            .unwrap_or_else(|| "size unknown".to_string())
                    )
                };
                lines.push(Line::from(format!(
                    "{marker} {kind:<7} {}{}",
                    item.name, metadata
                )));
            }
            lines.push(Line::from(""));
        }
        if !has_items {
            lines.push(Line::from("This folder is empty."));
            lines.push(Line::from(""));
        }
        lines.push(Line::from(
            "[j/k] choose  [enter] open/preview  [b/esc] parent  [y] report path  [r] reload",
        ));
    }
    if let Some(status) = &files.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    render_box(frame, area, title, lines);
}

fn display_file_path(path: &str) -> String {
    if path.is_empty() {
        "Server Root".to_string()
    } else {
        format!("Server Root / {path}")
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KiB", bytes as f64 / 1024.0);
    }
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

fn render_settings_lines(lines: &mut Vec<Line<'static>>, settings: &SettingsState) {
    lines.push(Line::from("SERVER SETTINGS · SCHEMA FROM AGENT"));
    if let Some(error) = &settings.error {
        lines.push(Line::from(format!("Settings unavailable: {error}")));
    } else if !settings.loaded {
        lines.push(Line::from("Loading settings…"));
    } else if let Some(response) = &settings.response {
        lines.push(Line::from(format!(
            "{} · {} · {}",
            response.server_name,
            response.server_type,
            if response.editable {
                "editable"
            } else {
                "read-only"
            }
        )));
        for (index, section) in response.sections.iter().enumerate() {
            let marker = if index == settings.selected_section {
                "›"
            } else {
                " "
            };
            lines.push(Line::from(format!(
                "{marker} {} [{}]",
                section.title,
                index + 1
            )));
        }
        if let Some(section) = response.sections.get(settings.selected_section) {
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "{} · [j/k] field  [enter] edit",
                section.title
            )));
            for (index, field) in section.fields.iter().enumerate() {
                let marker = if index == settings.selected_field {
                    "›"
                } else {
                    " "
                };
                let unit = field.unit.as_deref().unwrap_or("");
                lines.push(Line::from(format!(
                    "{marker} {} = {}{}",
                    field.label, field.value, unit
                )));
                if index == settings.selected_field {
                    if let Some(help_id) = &field.help_id {
                        lines.push(Line::from(format!("  Help: {help_id}")));
                    }
                    if let Some(options) = &field.options {
                        lines.push(Line::from(format!(
                            "  Options: {}",
                            options
                                .iter()
                                .map(|option| option.label.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )));
                    }
                }
            }
        }
    }
    if let Some((key, value)) = &settings.input {
        lines.push(Line::from(format!("Edit {key}: {value}_")));
    }
    if let Some(status) = &settings.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
}

fn render_connections_lines(
    lines: &mut Vec<Line<'static>>,
    connections: &super::connections::ConnectionsState,
) {
    lines.push(Line::from("CONNECTIONS · DIAGNOSTICS AND SERVICES"));
    lines.push(Line::from("[1] connection  [2] services  [r] reload"));
    if let Some(error) = &connections.error {
        lines.push(Line::from(format!("Connections unavailable: {error}")));
        return;
    }
    match connections.surface {
        ConnectionSurface::Connection => {
            if let Some(connection) = &connections.connectivity {
                lines.push(Line::from(format!(
                    "{} · {} · {}",
                    connection.headline, connection.server_type, connection.severity
                )));
                lines.push(Line::from(format!(
                    "Visibility: {}",
                    if connection.join_address.is_none() {
                        "HIDDEN"
                    } else if connection.externally_reachable == Some(true)
                        || connection
                            .join_address_source
                            .to_ascii_lowercase()
                            .contains("public")
                    {
                        "PUBLIC"
                    } else {
                        "LOCAL"
                    }
                )));
                lines.push(Line::from(format!(
                    "Join: {} · method: {}",
                    connection
                        .join_address
                        .as_deref()
                        .unwrap_or("not advertised"),
                    connection.method
                )));
                lines.push(Line::from(format!(
                    "Local port: {} · Public port: {}",
                    connection.port_diagnostics.local.outcome,
                    connection.port_diagnostics.public.outcome
                )));
                if let Some(detail) = &connection.detail {
                    lines.push(Line::from(format!("Detail: {detail}")));
                }
                if let Some(help_id) = &connection.help_id {
                    lines.push(Line::from(format!("Help: {help_id}")));
                }
            } else {
                lines.push(Line::from("Connectivity information unavailable."));
            }
            let duckdns = connections
                .duckdns
                .as_ref()
                .and_then(|status| status.hostname.as_deref())
                .unwrap_or("not configured");
            lines.push(Line::from(format!("DuckDNS: {duckdns}  [d] edit")));
            lines.push(Line::from(
                "Java and Bedrock join addresses retain their agent labels.",
            ));
        }
        ConnectionSurface::Services => {
            let playit = connections.playit.as_ref().map_or("unavailable", |status| {
                if status.is_running {
                    "RUNNING"
                } else {
                    "STOPPED"
                }
            });
            let broadcast = connections
                .broadcast
                .as_ref()
                .map_or("unavailable", |status| {
                    if status.xbox_broadcast_running {
                        "RUNNING"
                    } else {
                        "STOPPED"
                    }
                });
            let autostart = connections
                .broadcast_autostart
                .as_ref()
                .map_or(
                    "unknown",
                    |status| if status.enabled { "ON" } else { "OFF" },
                );
            let credentials =
                connections
                    .broadcast_credentials
                    .as_ref()
                    .map_or("unavailable", |status| {
                        if status.has_password {
                            "stored"
                        } else {
                            "not stored"
                        }
                    });
            lines.push(Line::from(format!("Playit: {playit}  [p] start/stop")));
            lines.push(Line::from(format!(
                "Xbox Broadcast: {broadcast}  [x] start/stop"
            )));
            lines.push(Line::from(format!(
                "Broadcast autostart: {autostart}  [a] toggle"
            )));
            lines.push(Line::from(format!(
                "Broadcast credentials: {credentials}  [e] replace (write-only password)"
            )));
            lines.push(Line::from(
                "Management services are separate from literal Minecraft console commands.",
            ));
        }
    }
    if let Some((kind, value)) = &connections.input {
        let shown = if *kind == super::connections::ConnectionInputKind::BroadcastCredentials {
            value.chars().map(|_| '•').collect::<String>()
        } else {
            value.clone()
        };
        lines.push(Line::from(format!("{}: {}_", kind.prompt(), shown)));
    }
    if let Some(status) = &connections.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
}

fn render_health_lines(lines: &mut Vec<Line<'static>>, health: &HealthState) {
    lines.push(Line::from("HEALTH · DIAGNOSIS AND REPAIR"));
    if let Some(error) = &health.error {
        lines.push(Line::from(format!("Health unavailable: {error}")));
    } else if let Some(response) = &health.health {
        lines.push(Line::from(format!(
            "Overall: {} · {} · {}",
            response.overall_severity,
            response.server_name,
            if response.server_running {
                "running"
            } else {
                "stopped"
            }
        )));
        for card in &response.cards {
            lines.push(Line::from(format!("{}: {}", card.title, card.severity)));
            if let Some(detail) = &card.detail {
                lines.push(Line::from(format!("  {detail}")));
            }
            if let Some(help_id) = &card.help_id {
                lines.push(Line::from(format!("  Help: {help_id}")));
            }
        }
    }
    if let Some(problems) = &health.problems {
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Startup problems: {}",
            problems.problems.len()
        )));
        if problems.problems.is_empty() {
            lines.push(Line::from(
                problems
                    .note
                    .clone()
                    .unwrap_or_else(|| "No repair needed.".to_string()),
            ));
        } else if health.detail_open {
            if let Some(problem) = health.selected_problem() {
                lines.push(Line::from(format!(
                    "› {} · {}",
                    problem.kind_title, problem.offender_name
                )));
                lines.push(Line::from(problem.raw_excerpt.clone()));
                for (index, action) in problem.available_actions.iter().enumerate() {
                    lines.push(Line::from(format!("[{}] repair: {action}", index + 1)));
                }
            }
            lines.push(Line::from("[esc] back"));
        } else {
            for (index, problem) in problems.problems.iter().enumerate() {
                let marker = if index == health.selected_problem {
                    "›"
                } else {
                    " "
                };
                lines.push(Line::from(format!(
                    "{marker} {} · {}",
                    problem.kind_title, problem.offender_name
                )));
            }
            lines.push(Line::from("[j/k] choose  [enter] details  [r] reload"));
        }
    }
}

fn render_access_lines(lines: &mut Vec<Line<'static>>, access: &super::access::AccessState) {
    lines.push(Line::from("ACCESS · PLAYER ALLOWLIST AND NAMED TOKENS"));
    lines.push(Line::from(
        "[1] allowlist  [2] named users  [3] this credential  [r] reload",
    ));
    if let Some(error) = &access.error {
        lines.push(Line::from(format!("Access unavailable: {error}")));
        return;
    }
    match access.surface {
        AccessSurface::Allowlist => {
            lines.push(Line::from("Bedrock player access · [a] add  [x] remove"));
            if access.allowlist.is_empty() {
                lines.push(Line::from("No allowlist entries returned."));
            } else {
                for (index, entry) in access.allowlist.iter().enumerate() {
                    lines.push(Line::from(format!(
                        "{} {}",
                        if index == access.selected { "›" } else { " " },
                        entry.name
                    )));
                }
            }
        }
        AccessSurface::Users => {
            lines.push(Line::from(
                "Named users · admin permission required · [x] revoke",
            ));
            if access.users.is_empty() {
                lines.push(Line::from(
                    "No named users returned, or this credential is not an admin.",
                ));
            } else {
                for (index, user) in access.users.iter().enumerate() {
                    lines.push(Line::from(format!(
                        "{} {} · {} · {}{}",
                        if index == access.selected { "›" } else { " " },
                        user.label,
                        user.role,
                        if user.is_expired { "EXPIRED" } else { "active" },
                        user.expires_at_iso8601
                            .as_deref()
                            .map(|value| format!(" · expires {value}"))
                            .unwrap_or_default()
                    )));
                }
            }
        }
        AccessSurface::Me => {
            if let Some(identity) = &access.identity {
                lines.push(Line::from(format!(
                    "Credential: {} · role: {}",
                    identity.name, identity.role
                )));
                lines.push(Line::from(format!(
                    "Named token: {}",
                    identity.is_named_token
                )));
                lines.push(Line::from(format!(
                    "Permissions: {}",
                    identity.permissions.join(", ")
                )));
            } else {
                lines.push(Line::from("Credential identity unavailable."));
            }
        }
    }
    if let Some(value) = &access.input {
        lines.push(Line::from(format!("Add Bedrock player: {value}_")));
    }
    if let Some(status) = &access.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
}

fn render_manage_servers(frame: &mut Frame, area: Rect, manage: &ManageServersState, app: &App) {
    let title = if app.focus() == FocusTarget::Content {
        "› MANAGE SERVERS"
    } else {
        "MANAGE SERVERS"
    };
    let mut lines = Vec::new();
    match manage.surface {
        Some(ManageSurface::List) => {
            lines.push(Line::from("REGISTERED SERVERS"));
            lines.push(Line::from(
                "The agent owns these registrations and lifecycle states.",
            ));
            lines.push(Line::from(""));
            if manage.servers.is_empty() {
                lines.push(Line::from("No registered servers."));
            } else {
                for (index, server) in manage.servers.iter().enumerate() {
                    let marker = if index == manage.selected { "›" } else { " " };
                    let active = if app.overview().selected_server_id.as_deref() == Some(&server.id)
                    {
                        "ACTIVE"
                    } else {
                        "      "
                    };
                    lines.push(Line::from(format!(
                        "{marker} {active}  {}  {}  {}",
                        server.name, server.server_type, server.directory
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(
                "[j/k] choose  [enter] detail  [c] create  [i] import  [r] reload  [esc] close",
            ));
        }
        Some(ManageSurface::Detail) => {
            if let Some(server) = manage.selected_server() {
                let active = app.overview().selected_server_id.as_deref() == Some(&server.id);
                lines.push(Line::from(format!(
                    "{}  ·  {}  ·  {}",
                    server.name,
                    server.server_type,
                    if active { "ACTIVE" } else { "SAVED" }
                )));
                lines.push(Line::from(format!("Server ID: {}", server.id)));
                lines.push(Line::from(format!("Host path: {}", server.directory)));
                lines.push(Line::from(format!(
                    "Ports: game {} · bedrock {}",
                    server
                        .game_port
                        .map_or_else(|| "—".to_string(), |p| p.to_string()),
                    server
                        .bedrock_port
                        .map_or_else(|| "—".to_string(), |p| p.to_string())
                )));
                lines.push(Line::from(format!(
                    "Services: Playit {} · Xbox Broadcast {}",
                    on_off(server.playit_enabled),
                    on_off(server.xbox_broadcast_enabled)
                )));
                lines.push(Line::from(
                    "EULA: accept through the agent · Delete: removes managed server data",
                ));
                lines.push(Line::from(""));
                lines.push(Line::from(
                    "[e] editor  [a] set active  [n] rename  [u] accept EULA  [d] delete  [esc] list",
                ));
            }
        }
        Some(ManageSurface::Create) => {
            lines.push(Line::from("CREATE SERVER · STAGED CHOICES"));
            lines.push(Line::from(format!(
                "Name: {}  Type: {}  Flavor: {}  Port: {}  World: {}  EULA: {}",
                empty_as_default(&manage.create_draft.name),
                manage.create_draft.server_type.as_deref().unwrap_or("java"),
                manage
                    .create_draft
                    .java_flavor
                    .as_deref()
                    .unwrap_or("default"),
                manage
                    .create_draft
                    .port
                    .map_or_else(|| "default".to_string(), |p| p.to_string()),
                manage
                    .create_draft
                    .world_name
                    .as_deref()
                    .unwrap_or("default"),
                if manage.create_draft.accept_eula {
                    "yes"
                } else {
                    "no"
                }
            )));
            lines.push(Line::from(
                "Choices are reviewed before the create request is sent.",
            ));
            if manage.create_step_is_review() {
                lines.push(Line::from("[enter] create server  [esc] cancel"));
            }
        }
        Some(ManageSurface::Import) => {
            lines.push(Line::from("IMPORT SERVER · STAGED CHOICES"));
            lines.push(Line::from(format!(
                "Source: {}  Type: {}  Name: {}  World: {}  EULA: {}",
                empty_as_default(&manage.import_draft.source_path),
                manage.import_draft.server_type.as_deref().unwrap_or("java"),
                manage
                    .import_draft
                    .display_name
                    .as_deref()
                    .unwrap_or("detected"),
                manage
                    .import_draft
                    .active_world_name
                    .as_deref()
                    .unwrap_or("detected"),
                if manage.import_draft.accept_eula {
                    "yes"
                } else {
                    "no"
                }
            )));
            lines.push(Line::from(
                "The source path is interpreted by the agent host.",
            ));
            if manage.import_step_is_review() {
                lines.push(Line::from("[enter] import server  [esc] cancel"));
            }
        }
        None => {}
    }
    if let Some((kind, value)) = &manage.input {
        lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
    }
    if let Some(status) = &manage.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    render_box(frame, area, title, lines);
}

fn render_server_editor(frame: &mut Frame, area: Rect, editor: &ServerEditorState) {
    let mut lines = vec![Line::from(format!(
        "{}  ·  {}",
        editor.server_name(),
        editor
            .server
            .as_ref()
            .map_or("unknown", |server| server.server_type.as_str())
    ))];
    lines.push(Line::from(
        "[1] General  [2] Services  [3] Java  [r] reload  [esc] back",
    ));
    lines.push(Line::from(""));
    match editor.surface {
        EditorSurface::General => {
            if let Some(server) = &editor.server {
                lines.push(Line::from("GENERAL"));
                lines.push(Line::from(format!(
                    "Display name: {}  [n] rename",
                    server.name
                )));
                lines.push(Line::from(format!(
                    "Directory: {}  [p] edit path",
                    server.directory
                )));
                let ram = editor.ram.as_ref().map_or_else(
                    || "unavailable".to_string(),
                    |ram| format!("{}–{} GB", ram.min_ram_gb, ram.max_ram_gb),
                );
                lines.push(Line::from(format!("RAM: {ram}  [m] edit min|max GB")));
                lines.push(Line::from(format!(
                    "Ports: game {} · bedrock {}  [o] edit game port",
                    server
                        .game_port
                        .map_or_else(|| "—".to_string(), |p| p.to_string()),
                    server
                        .bedrock_port
                        .map_or_else(|| "—".to_string(), |p| p.to_string())
                )));
                lines.push(Line::from(format!(
                    "Storage: {}",
                    editor.storage_bytes.map_or_else(
                        || "unavailable".to_string(),
                        |bytes| format_bytes(bytes as f64)
                    )
                )));
                lines.push(Line::from(
                    "EULA: agent-managed acceptance in Manage Servers",
                ));
                lines.push(Line::from(
                    "Deletion: Manage Servers confirms removal of managed data",
                ));
            }
        }
        EditorSurface::Services => {
            lines.push(Line::from("SERVICES · CAPABILITY-BACKED"));
            lines.push(Line::from(format!(
                "Playit: {} · {}",
                if editor.playit_available {
                    "available"
                } else {
                    "unavailable"
                },
                editor
                    .playit
                    .as_ref()
                    .map_or("state unavailable".to_string(), |status| {
                        if status.is_running {
                            "RUNNING [p] stop"
                        } else {
                            "STOPPED [p] start"
                        }
                        .to_string()
                    })
            )));
            lines.push(Line::from(format!(
                "Xbox Broadcast: {} · {}",
                if editor.broadcast_available {
                    "available"
                } else {
                    "unavailable"
                },
                editor
                    .broadcast
                    .as_ref()
                    .map_or("state unavailable".to_string(), |status| {
                        if status.xbox_broadcast_running {
                            "RUNNING [x] stop"
                        } else {
                            "STOPPED [x] start"
                        }
                        .to_string()
                    })
            )));
            lines.push(Line::from(
                "Controls appear only when host capability and token permission allow them.",
            ));
        }
        EditorSurface::Java => {
            lines.push(Line::from("JAVA · DETECTED RUNTIMES"));
            lines.push(Line::from(format!(
                "Configured path: {}  [p] edit path",
                editor
                    .java_config
                    .as_ref()
                    .and_then(|config| config.executable_path.as_deref())
                    .unwrap_or("default")
            )));
            lines.push(Line::from(format!(
                "Extra arguments: {}  [a] edit",
                editor
                    .java_config
                    .as_ref()
                    .and_then(|config| config.extra_flags.as_deref())
                    .unwrap_or("none")
            )));
            if editor.java_runtimes.is_empty() {
                lines.push(Line::from("No Java runtimes detected. [d] check again"));
            } else {
                for (index, runtime) in editor.java_runtimes.iter().enumerate() {
                    lines.push(Line::from(format!(
                        "{} {} · {} · {}",
                        if index == editor.selected_runtime {
                            "›"
                        } else {
                            " "
                        },
                        runtime.name,
                        runtime
                            .major_version
                            .map_or_else(|| "Java ?".to_string(), |major| format!("Java {major}")),
                        runtime.executable_path
                    )));
                }
                lines.push(Line::from(
                    "[j/k] choose runtime  [enter] use it  [d] report detection",
                ));
            }
        }
    }
    if let Some((kind, value)) = &editor.input {
        lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
    }
    if let Some(status) = &editor.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    if let Some(settings) = &editor.settings {
        lines.push(Line::from(format!(
            "Settings schema: {} field(s) · {}",
            settings
                .sections
                .iter()
                .map(|section| section.fields.len())
                .sum::<usize>(),
            if settings.editable {
                "editable"
            } else {
                "read-only"
            }
        )));
    }
    if let Some(error) = &editor.error {
        lines.push(Line::from(format!("Editor data note: {error}")));
    }
    render_box(frame, area, "› SERVER EDITOR", lines);
}

fn empty_as_default(value: &str) -> &str {
    if value.is_empty() { "(not set)" } else { value }
}

fn on_off(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "ON",
        Some(false) => "OFF",
        None => "unknown",
    }
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

fn render_components(frame: &mut Frame, area: Rect, components: &ComponentsState) {
    let mut lines = vec![Line::from(
        "[1] JAR/version  [2] add-ons  [3] system  [4] catalog  [5] packs  [6] modpacks",
    )];
    if let Some(error) = &components.error {
        lines.push(Line::from(format!("Components unavailable: {error}")));
    } else if !components.loaded {
        lines.push(Line::from(
            "Loading component state from the selected server…",
        ));
    } else {
        lines.push(Line::from(format!(
            "{}  ·  {} item(s)",
            components.surface.label(),
            components.item_count()
        )));
        match components.surface {
            ComponentSurface::Versions => render_versions(&mut lines, components),
            ComponentSurface::Addons => render_addons(&mut lines, components),
            ComponentSurface::System => render_system_components(&mut lines, components),
            ComponentSurface::Catalog => render_catalog(&mut lines, components),
            ComponentSurface::ResourcePacks => render_resource_packs(&mut lines, components),
            ComponentSurface::Modpacks => {
                lines.push(Line::from(
                    "Modpack archives use bounded staging before inspection or import.",
                ));
                lines.push(Line::from(
                    "[i] enter: inspect|import|replace | local .mrpack/.zip path",
                ));
            }
        }
        if let Some((kind, value)) = &components.input {
            lines.push(Line::from(format!("{}: {}_", kind.prompt(), value)));
        }
        if let Some(status) = &components.status {
            lines.push(Line::from(format!("Status: {status}")));
        }
    }
    render_box(frame, area, "› COMPONENTS", lines);
}

fn render_versions(lines: &mut Vec<Line<'static>>, components: &ComponentsState) {
    let Some(versions) = &components.versions else {
        lines.push(Line::from("Server version information is unavailable."));
        return;
    };
    lines.push(Line::from(format!(
        "Flavor: {}  · current: {}  · supports changes: {}",
        versions.flavor_name,
        versions
            .current_version
            .as_deref()
            .unwrap_or("not reported"),
        versions.supports_versions
    )));
    if components.detail_open {
        if let Some(version) = components.selected_version() {
            lines.push(Line::from(format!(
                "SELECTED  {}  · {}  · {}",
                version.display_label,
                version.mc_version,
                if version.is_latest {
                    "LATEST"
                } else {
                    "available"
                }
            )));
            lines.push(Line::from(format!("Version ID: {}", version.id)));
            lines.push(Line::from("[a] actions  [esc] version list"));
            if components.action_menu_open {
                lines.push(Line::from(
                    "ACTIONS  [v] change this server's version  [esc] close",
                ));
            }
        }
    } else {
        for (index, version) in versions.versions.iter().enumerate().take(16) {
            lines.push(Line::from(format!(
                "{} {}  {}{}",
                if index == components.selected {
                    "›"
                } else {
                    " "
                },
                version.display_label,
                version.mc_version,
                if version.is_latest { "  LATEST" } else { "" }
            )));
        }
        lines.push(Line::from("[j/k] choose  [enter] detail  [r] reload"));
    }
}

fn render_addons(lines: &mut Vec<Line<'static>>, components: &ComponentsState) {
    let Some(addons) = &components.addons else {
        lines.push(Line::from("Installed add-ons are unavailable."));
        return;
    };
    if addons.pack_managed == Some(true) {
        lines.push(Line::from(format!(
            "PACK-MANAGED  {}  · individual add-on updates may be refused",
            addons.pack_name.as_deref().unwrap_or("unnamed pack")
        )));
    }
    if let Some(note) = &addons.note {
        lines.push(Line::from(format!("Agent note: {note}")));
    }
    if components.detail_open {
        if let Some(addon) = components.selected_addon() {
            lines.push(Line::from(format!(
                "SELECTED  {}  · {}",
                addon.display_name,
                if addon.is_enabled {
                    "ENABLED"
                } else {
                    "DISABLED"
                }
            )));
            lines.push(Line::from(format!("Jar stem: {}", addon.jar_stem)));
            lines.push(Line::from(format!(
                "Version: {}  · available: {}  · bucket: {}",
                addon.current_version.as_deref().unwrap_or("not reported"),
                addon.available_version.as_deref().unwrap_or("not reported"),
                addon.bucket
            )));
            lines.push(Line::from("[a] actions  [esc] add-on list"));
            if components.action_menu_open {
                lines.push(Line::from(
                    "ACTIONS  [u] update  [e] enable  [d] disable  [x] remove",
                ));
            }
        }
    } else {
        if addons.addons.is_empty() {
            lines.push(Line::from("No installed add-ons are reported."));
        }
        for (index, addon) in addons.addons.iter().enumerate().take(16) {
            lines.push(Line::from(format!(
                "{} {}  {}  {}",
                if index == components.selected {
                    "›"
                } else {
                    " "
                },
                addon.display_name,
                if addon.is_enabled {
                    "ENABLED"
                } else {
                    "DISABLED"
                },
                addon
                    .current_version
                    .as_deref()
                    .unwrap_or("version unknown")
            )));
        }
        lines.push(Line::from("[j/k] choose  [enter] detail  [r] reload"));
    }
}

fn render_system_components(lines: &mut Vec<Line<'static>>, components: &ComponentsState) {
    let Some(system) = &components.system else {
        lines.push(Line::from("System component status is unavailable."));
        return;
    };
    lines.push(Line::from(format!(
        "Restart required to apply: {}",
        system.restart_required_to_apply
    )));
    if components.detail_open {
        if let Some(component) = components.selected_system() {
            lines.push(Line::from(format!("SELECTED  {}", component.name)));
            lines.push(Line::from(format!(
                "Installed: {}  · latest: {}  · up to date: {}",
                component
                    .installed_version
                    .as_deref()
                    .unwrap_or("not reported"),
                component
                    .latest_version
                    .as_deref()
                    .unwrap_or("not reported"),
                component.is_up_to_date
            )));
            if let Some(note) = &component.note {
                lines.push(Line::from(format!("Agent note: {note}")));
            }
            lines.push(Line::from("[a] actions  [esc] system component list"));
            if components.action_menu_open {
                lines.push(Line::from(
                    "ACTIONS  [u] update this component  [esc] close",
                ));
            }
        }
    } else {
        for (index, component) in system.components.iter().enumerate().take(16) {
            lines.push(Line::from(format!(
                "{} {}  {} → {}",
                if index == components.selected {
                    "›"
                } else {
                    " "
                },
                component.name,
                component
                    .installed_label
                    .as_deref()
                    .unwrap_or("not installed"),
                component
                    .latest_version
                    .as_deref()
                    .unwrap_or("latest unknown")
            )));
        }
        lines.push(Line::from("[j/k] choose  [enter] detail  [r] reload"));
    }
}

fn render_catalog(lines: &mut Vec<Line<'static>>, components: &ComponentsState) {
    let Some(catalog) = &components.catalog else {
        lines.push(Line::from("[ / ] search the server-compatible catalog."));
        return;
    };
    if let Some(note) = &catalog.note {
        lines.push(Line::from(format!("Provider note: {note}")));
    }
    if components.detail_open {
        if let Some(item) = components.selected_catalog() {
            lines.push(Line::from(format!(
                "SELECTED  {}  · {}",
                item.title, item.project_type
            )));
            lines.push(Line::from(format!(
                "Project: {}  · author: {}",
                item.project_id, item.author
            )));
            lines.push(Line::from(format!(
                "Downloads: {}  · client-only: {}",
                item.downloads, item.is_client_only
            )));
            lines.push(Line::from(item.description.clone()));
            lines.push(Line::from("[a] actions  [esc] catalog list"));
            if components.action_menu_open {
                lines.push(Line::from(
                    "ACTIONS  [i] install selected add-on  [esc] close",
                ));
            }
        }
    } else {
        if catalog.results.is_empty() {
            lines.push(Line::from("No catalog results match the current search."));
        }
        for (index, item) in catalog.results.iter().enumerate().take(16) {
            lines.push(Line::from(format!(
                "{} {}  {}  {} downloads",
                if index == components.selected {
                    "›"
                } else {
                    " "
                },
                item.title,
                item.project_type,
                item.downloads
            )));
        }
        lines.push(Line::from("[/] search  [j/k] choose  [enter] detail"));
    }
}

fn render_resource_packs(lines: &mut Vec<Line<'static>>, components: &ComponentsState) {
    let Some(packs) = &components.resource_packs else {
        lines.push(Line::from("Resource-pack state is unavailable."));
        return;
    };
    if let Some(note) = &packs.note {
        lines.push(Line::from(format!("Agent note: {note}")));
    }
    lines.push(Line::from(format!(
        "Java packs: {}  · Geyser packs: {}  · required: {}",
        packs.packs.len(),
        packs.geyser_packs.len(),
        packs.require_pack
    )));
    if components.detail_open {
        if let Some(pack) = components.selected_resource_pack() {
            lines.push(Line::from(format!(
                "SELECTED  {}  · {}",
                pack.name, pack.type_label
            )));
            lines.push(Line::from(format!(
                "File: {}  · size: {}",
                pack.file_name, pack.file_size_display
            )));
            lines.push(Line::from(format!(
                "State: {}",
                if pack.is_active {
                    "ACTIVE"
                } else {
                    "AVAILABLE"
                }
            )));
            lines.push(Line::from("[a] actions  [esc] resource-pack list"));
            if components.action_menu_open {
                lines.push(Line::from(
                    "ACTIONS  [a] activate  [c] clear active  [u] set URL  [x] remove",
                ));
            }
        }
    } else {
        for (index, pack) in packs
            .packs
            .iter()
            .chain(packs.geyser_packs.iter())
            .enumerate()
            .take(16)
        {
            lines.push(Line::from(format!(
                "{} {}  {}  {}",
                if index == components.selected {
                    "›"
                } else {
                    " "
                },
                pack.name,
                if pack.is_active {
                    "ACTIVE"
                } else {
                    "AVAILABLE"
                },
                pack.pack_kind
            )));
        }
        lines.push(Line::from("[j/k] choose  [enter] detail  [r] reload"));
    }
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
    if let Some(editor) = app.editor() {
        render_server_editor(frame, area, editor);
        return;
    }
    if app.manage_servers().is_open() {
        render_manage_servers(frame, area, app.manage_servers(), app);
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
            if app.active_tab() == 4 {
                render_components(frame, area, app.components());
                return;
            }
            if app.active_tab() == 5 {
                render_admin(frame, area, app);
                return;
            }
            if app.active_tab() == 6 {
                render_files(frame, area, app, app.files());
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

fn render_support(frame: &mut Frame, area: Rect, app: &App) {
    let width = area.width.saturating_sub(6).clamp(44, 104);
    let height = area.height.saturating_sub(4).clamp(12, area.height.max(12));
    let modal = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, modal);
    match app.support_surface() {
        Some(SupportSurface::Help) => render_support_box(
            frame,
            modal,
            "› KEYBOARD HELP",
            vec![
                Line::from("The first session keeps the important paths close to the keyboard."),
                Line::from(""),
                Line::from("[g] Handbook and router guides"),
                Line::from("[A] Agent status, pairing, and reconnect"),
                Line::from("[,] MSC settings and reset boundaries"),
                Line::from("[1-7] server sections  [m] Manage Servers"),
                Line::from("[i] activity  [a/x] start or stop the selected server"),
                Line::from("[Tab] move focus  [Esc] close this surface  [q] quit"),
                Line::from(""),
                Line::from(
                    "Raw console input stays literal Minecraft text; management requests use the API.",
                ),
            ],
        ),
        Some(SupportSurface::Agent) => render_agent(frame, modal, app),
        Some(SupportSurface::Handbook) => render_handbook(frame, modal, app),
        Some(SupportSurface::AppSettings) => render_app_settings(frame, modal, app),
        None => {}
    }
}

fn render_support_box(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_agent(frame: &mut Frame, area: Rect, app: &App) {
    let agent = app.agent();
    let mut lines = vec![Line::from(
        "[1] status  [2] pairing  [I] install  [S] start  [X] stop  [R] reconnect  [F] repair",
    )];
    if let Some(error) = &agent.error {
        lines.push(Line::from(format!("Error: {error}")));
    }
    match agent.surface {
        AgentSurface::Status => {
            lines.push(Line::from(format!(
                "Local service: {}  PID: {}",
                agent.service.state.label(),
                agent
                    .service
                    .pid
                    .map_or("—".to_string(), |pid| pid.to_string())
            )));
            lines.push(Line::from(format!(
                "Platform: {}  Name: {}",
                agent.service.platform, agent.service.service_name
            )));
            lines.push(Line::from(agent.service.detail.clone()));
            if let Some(identity) = &agent.identity {
                lines.push(Line::from(format!("")));
                lines.push(Line::from(format!(
                    "Host session: {} ({})",
                    identity.name, identity.role
                )));
                lines.push(Line::from(format!(
                    "Permissions: {}",
                    identity
                        .permissions
                        .iter()
                        .map(|p| format!("{p:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(
                "The service runs independently of this terminal window.",
            ));
        }
        AgentSurface::Pairing => {
            lines.push(Line::from(
                "Create a one-use desktop pairing code, or exchange a code from another MSC host.",
            ));
            lines.push(Line::from(
                "[c] create code  [p] type pairing code  [enter] exchange  [r] refresh",
            ));
            if let Some(input) = &agent.pairing_input {
                lines.push(Line::from(format!("Pairing code: {input}")));
            }
            if let Some(code) = &agent.pairing_code {
                lines.push(Line::from(format!("New one-use code: {code}")));
                lines.push(Line::from(format!(
                    "Host id: {}",
                    agent.pairing_host_id.as_deref().unwrap_or("—")
                )));
                lines.push(Line::from(format!(
                    "Expires: {}",
                    agent.pairing_expires_at.as_deref().unwrap_or("—")
                )));
                lines.push(Line::from(
                    "The resulting credential is held in memory only.",
                ));
            }
        }
    }
    if let Some(status) = &agent.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    render_support_box(frame, area, "› AGENT SUPPORT", lines);
}

fn render_handbook(frame: &mut Frame, area: Rect, app: &App) {
    let handbook = app.handbook();
    let mut lines = vec![Line::from(
        "[1] topics  [s] search  [r] router guides  [t] troubleshooting  [esc] close",
    )];
    if let Some(error) = &handbook.error {
        lines.push(Line::from(format!("Error: {error}")));
    }
    match handbook.surface {
        HandbookSurface::Topics => {
            lines.push(Line::from(format!("Search: {}", handbook.query)));
            if let Some(input) = &handbook.search_input {
                lines.push(Line::from(format!("Search input: {input}")));
            }
            for (index, topic) in handbook.filtered_topics().iter().take(18).enumerate() {
                lines.push(Line::from(format!(
                    "{} {}  [{}]",
                    if index == handbook.selected {
                        "›"
                    } else {
                        " "
                    },
                    topic.title,
                    topic.category
                )));
            }
            lines.push(Line::from("[j/k] choose  [enter] read selected topic"));
        }
        HandbookSurface::Topic => {
            if let Some(topic) = &handbook.topic {
                lines.push(Line::from(format!("{}  [{}]", topic.title, topic.category)));
                if let Some(subtitle) = &topic.subtitle {
                    lines.push(Line::from(subtitle.clone()));
                }
                if let Some(analogy) = &topic.analogy {
                    lines.push(Line::from(format!("Analogy: {analogy}")));
                }
                lines.push(Line::from(""));
                lines.extend(topic.body.lines().map(|line| Line::from(line.to_string())));
                for section in &topic.sections {
                    append_help_block(&mut lines, section);
                }
                let related = handbook.related_titles();
                if !related.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(format!("Related: {}", related.join("  ·  "))));
                }
                lines.push(Line::from(
                    "[b] topics  [j/k] related topic  [enter] open related",
                ));
            }
        }
        HandbookSurface::RouterSearch => {
            lines.push(Line::from(
                "Search a provider, router, mesh system, or model.",
            ));
            if let Some(input) = &handbook.search_input {
                lines.push(Line::from(format!("Router search: {input}")));
            } else {
                lines.push(Line::from(format!(
                    "Query: {}  [s] type a query",
                    handbook.query
                )));
            }
            for (index, (_id, title)) in handbook.router_candidates().iter().enumerate().take(12) {
                lines.push(Line::from(format!(
                    "{} {}",
                    if index == handbook.router_selected {
                        "›"
                    } else {
                        " "
                    },
                    title
                )));
            }
            lines.push(Line::from(
                "[j/k] choose  [o] open selected guide  [t] troubleshoot",
            ));
        }
        HandbookSurface::RouterGuide => {
            if let Some(guide) = &handbook.router_guide {
                append_json_text(&mut lines, guide.get("guide"));
                append_json_text(&mut lines, guide.get("runtime"));
                if let Some(sections) = guide.get("sections").and_then(Value::as_array) {
                    for section in sections {
                        append_json_text(&mut lines, section.get("title"));
                        append_json_text(&mut lines, section.get("items"));
                    }
                }
                lines.push(Line::from("[b] router search  [t] troubleshooting"));
            }
        }
        HandbookSurface::Troubleshooting => {
            lines.push(Line::from(
                "Select symptoms, then ask the agent for likely causes and next actions.",
            ));
            for (index, (id, title)) in handbook.symptoms().iter().enumerate().take(16) {
                let selected = handbook.selected_symptoms.contains(id);
                lines.push(Line::from(format!(
                    "{} [{}] {}",
                    if index == handbook.selected {
                        "›"
                    } else {
                        " "
                    },
                    if selected { "x" } else { " " },
                    title
                )));
            }
            if let Some(analysis) = &handbook.troubleshooting {
                lines.push(Line::from(""));
                append_json_text(&mut lines, analysis.get("summary"));
                append_json_text(&mut lines, analysis.get("recommendedActions"));
            }
            lines.push(Line::from(
                "[j/k] choose  [space] select  [enter] analyze  [esc] back",
            ));
        }
    }
    render_support_box(frame, area, "› SERVER HANDBOOK", lines);
}

fn append_help_block(lines: &mut Vec<Line<'static>>, block: &Value) {
    let kind = block
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("content");
    lines.push(Line::from(format!("{kind}:")));
    for key in ["markdown", "text", "phase"] {
        if let Some(value) = block.get(key).and_then(Value::as_str) {
            lines.extend(value.lines().map(|line| Line::from(format!("  {line}"))));
        }
    }
    for key in ["items", "bullets", "steps"] {
        if let Some(values) = block.get(key).and_then(Value::as_array) {
            for value in values.iter().take(20) {
                lines.push(Line::from(format!("  • {}", value_to_text(value))));
            }
        }
    }
}

fn append_json_text(lines: &mut Vec<Line<'static>>, value: Option<&Value>) {
    let Some(value) = value else { return };
    match value {
        Value::String(text) => lines.extend(text.lines().map(|line| Line::from(line.to_string()))),
        Value::Array(values) => values.iter().take(24).for_each(|value| {
            lines.push(Line::from(format!("  • {}", value_to_text(value))));
        }),
        Value::Object(map) => {
            if let Some(title) = map.get("displayName").and_then(Value::as_str) {
                lines.push(Line::from(title.to_string()));
            } else if let Some(title) = map.get("title").and_then(Value::as_str) {
                lines.push(Line::from(title.to_string()));
            }
        }
        _ => {}
    }
}

fn value_to_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| serde_json::to_string(value).unwrap_or_else(|_| "—".to_string()))
}

fn render_app_settings(frame: &mut Frame, area: Rect, app: &App) {
    let settings = app.app_settings();
    let mut lines = vec![Line::from(
        "Terminal-local preferences are separate from the authenticated host reset.",
    )];
    if settings.surface == AppSettingsSurface::Preferences {
        lines.push(Line::from(format!(
            "[1] first-session guide: {}",
            if settings.show_first_session_guide {
                "shown"
            } else {
                "hidden"
            }
        )));
        lines.push(Line::from(
            "[c] reset this client (notes and presentation preferences only)",
        ));
        lines.push(Line::from(
            "[2] reset host configuration  [3] reset host everything",
        ));
        lines.push(Line::from("[esc] close"));
    } else {
        lines.push(Line::from(format!("Host: {}", app.host())));
        lines.push(Line::from(format!(
            "Selected server: {}",
            app.overview().selected_server_name()
        )));
        lines.push(Line::from(format!(
            "Reset mode: {}",
            settings.host_reset_mode
        )));
        if let Some(input) = &settings.host_reset_confirmation {
            lines.push(Line::from(format!("Type exactly RESET AGENT: {input}")));
            lines.push(Line::from("[enter] submit  [esc] cancel"));
        }
        if let Some(result) = &settings.host_reset_result {
            lines.push(Line::from(format!("Operation: {}", result.operation_id)));
            lines.push(Line::from(format!(
                "State after completion: {}",
                result.agent_state
            )));
            lines.push(Line::from(result.message.clone()));
            lines.push(Line::from(
                "The old credential cannot be used; pair this host again.",
            ));
        }
    }
    if let Some(status) = &settings.status {
        lines.push(Line::from(format!("Status: {status}")));
    }
    if let Some(error) = &settings.error {
        lines.push(Line::from(format!("Error: {error}")));
    }
    render_support_box(frame, area, "› MSC SETTINGS", lines);
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
