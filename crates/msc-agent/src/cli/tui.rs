//! Host-local terminal client. The first screen owns only presentation and
//! delegates service changes and server reads to the existing CLI paths.

mod create;

use std::io::{self, IsTerminal, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use msc_api::dto::{ActiveServerRequestDto, RemoteApiStatus, ServerDto, SimpleResultDto};
use msc_infrastructure::service::ServiceState;

use super::service::{self, AgentAction, LocalAgentStatus};
use super::transport::SharedClient;
use super::{CliError, CommonArgs};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self, CliError> {
        terminal::enable_raw_mode().map_err(terminal_error)?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen, Hide) {
            let _ = terminal::disable_raw_mode();
            return Err(terminal_error(error));
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), Show, ResetColor, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn terminal_error(error: io::Error) -> CliError {
    CliError::internal(format!("terminal: {error}"))
}

struct Snapshot {
    service: LocalAgentStatus,
    servers: Option<Vec<ServerDto>>,
    active_server_id: Option<String>,
    active_server_running: bool,
    connection_error: Option<String>,
}

struct ViewState<'a> {
    selected: usize,
    message: &'a str,
    confirm_stop: bool,
    manage_open: bool,
    manage_selected: usize,
    wizard: Option<&'a create::Wizard>,
}

async fn load() -> Result<Snapshot, CliError> {
    let service = service::local_agent_status()?;
    let (servers, active_server_id, active_server_running, connection_error) =
        if service.report.state == ServiceState::Running {
            match SharedClient::connect_local().await {
                Ok(client) => match client.get_json::<Vec<ServerDto>>("/v1/servers").await {
                    Ok(servers) => match client.get_json::<RemoteApiStatus>("/v1/status").await {
                        Ok(status) => {
                            (Some(servers), status.active_server_id, status.running, None)
                        }
                        Err(error) => (Some(servers), None, false, Some(error.to_string())),
                    },
                    Err(error) => (None, None, false, Some(error.to_string())),
                },
                Err(error) => (None, None, false, Some(error.to_string())),
            }
        } else {
            (None, None, false, None)
        };
    Ok(Snapshot {
        service,
        servers,
        active_server_id,
        active_server_running,
        connection_error,
    })
}

async fn select_server(server_id: &str) -> Result<(), CliError> {
    let client = SharedClient::connect_local().await?;
    let body = ActiveServerRequestDto {
        server_id: Some(server_id.to_string()),
    };
    let _: SimpleResultDto = client.post_json("/v1/active-server", &body).await?;
    Ok(())
}

pub async fn run(common: CommonArgs) -> Result<(), CliError> {
    if common.json {
        return Err(CliError::usage(
            "msc tui is interactive and does not support --json",
        ));
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::usage("msc tui requires an interactive terminal"));
    }
    let mut snapshot = load().await?;
    let host_name = std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "This computer".into());
    let _terminal = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let mut selected = 0usize;
    let mut manage_open = false;
    let mut manage_selected = 0usize;
    let mut wizard: Option<create::Wizard> = None;
    let mut message = String::new();
    let mut confirm_stop = false;
    let mut last_refresh = Instant::now();

    loop {
        draw(
            &mut stdout,
            &snapshot,
            &host_name,
            &ViewState {
                selected,
                message: &message,
                confirm_stop,
                manage_open,
                manage_selected,
                wizard: wizard.as_ref(),
            },
        )
        .map_err(terminal_error)?;
        if !event::poll(Duration::from_millis(250)).map_err(terminal_error)? {
            if !manage_open && wizard.is_none() && last_refresh.elapsed() >= REFRESH_INTERVAL {
                refresh(&mut snapshot, &mut message).await;
                last_refresh = Instant::now();
            }
            continue;
        }
        let Event::Key(key) = event::read().map_err(terminal_error)? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if confirm_stop {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    message = match service::change_local_agent(AgentAction::Stop) {
                        Ok(()) => "Agent service stopped.".into(),
                        Err(error) => error.to_string(),
                    };
                    confirm_stop = false;
                    refresh(&mut snapshot, &mut message).await;
                }
                _ => {
                    confirm_stop = false;
                    message = "Stop cancelled.".into();
                }
            }
            continue;
        }
        if let Some(active) = wizard.as_mut() {
            match active.handle(key, &mut stdout).await {
                create::Outcome::Stay => {}
                create::Outcome::Close => {
                    wizard = None;
                    manage_open = true;
                }
                create::Outcome::Created(note) => {
                    wizard = None;
                    message = note;
                    refresh(&mut snapshot, &mut message).await;
                    manage_open = false;
                }
            }
            continue;
        }
        if manage_open {
            let server_count = snapshot.servers.as_ref().map_or(0, Vec::len);
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => manage_open = false,
                KeyCode::Up => manage_selected = manage_selected.saturating_sub(1),
                KeyCode::Down => {
                    manage_selected = (manage_selected + 1).min(server_count.saturating_sub(1));
                }
                KeyCode::Char('t') => {
                    message = "Transfer is not connected yet; no server was changed.".into()
                }
                KeyCode::Char('c') => {
                    wizard = Some(create::Wizard::new());
                    manage_open = false;
                }
                KeyCode::Char('R') => {
                    refresh(&mut snapshot, &mut message).await;
                    manage_selected = manage_selected.min(
                        snapshot
                            .servers
                            .as_ref()
                            .map_or(0, |list| list.len().saturating_sub(1)),
                    );
                }
                KeyCode::Enter => {
                    if let Some(server) = snapshot
                        .servers
                        .as_ref()
                        .and_then(|list| list.get(manage_selected))
                    {
                        let id = server.id.clone();
                        let name = server.name.clone();
                        match select_server(&id).await {
                            Ok(()) => {
                                manage_open = false;
                                refresh(&mut snapshot, &mut message).await;
                                message = format!("Selected server: {name}");
                            }
                            Err(error) => message = error.to_string(),
                        }
                    } else {
                        message = "No server to select. Create Server is coming next.".into();
                    }
                }
                _ => {}
            }
            continue;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => break,
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down => selected = (selected + 1).min(4),
            KeyCode::Char('a') => selected = 0,
            KeyCode::Char('m') => {
                selected = 1;
                if snapshot.servers.is_some() {
                    manage_open = true;
                    manage_selected = 0;
                } else {
                    message = "Agent API unavailable; start the service or refresh its connection."
                        .into();
                }
            }
            KeyCode::Char('s') => selected = 2,
            KeyCode::Char('e') => selected = 3,
            KeyCode::Char('r') => selected = 4,
            KeyCode::Char('R') => {
                refresh(&mut snapshot, &mut message).await;
                last_refresh = Instant::now();
            }
            KeyCode::Char('?') => message = "Help screen is not connected yet.".into(),
            KeyCode::Char('S') => message = "Settings screen is not connected yet.".into(),
            KeyCode::Enter => {
                match selected {
                    0 => {
                        message = "Server workspace is not connected yet. Use Manage Servers next."
                            .into();
                    }
                    1 => {
                        if snapshot.servers.is_some() {
                            manage_open = true;
                            manage_selected = 0;
                        } else {
                            message = "Agent API unavailable; start the service or refresh its connection.".into();
                        }
                    }
                    2 => {
                        if snapshot.service.report.state == ServiceState::NotInstalled {
                            message = "Install the agent service before starting it.".into();
                        } else if snapshot.service.report.state == ServiceState::Running {
                            confirm_stop = true;
                            message = "Stop the agent service? Enter confirms; Esc cancels.".into();
                        } else {
                            message = match service::change_local_agent(AgentAction::Start) {
                                Ok(()) => "Agent service started.".into(),
                                Err(error) => error.to_string(),
                            };
                            refresh(&mut snapshot, &mut message).await;
                        }
                    }
                    3 => {
                        if let Some(enabled) = snapshot.service.boot_enabled {
                            let action = if enabled {
                                AgentAction::Disable
                            } else {
                                AgentAction::Enable
                            };
                            message = match service::change_local_agent(action) {
                                Ok(()) => if enabled {
                                    "Start at boot disabled."
                                } else {
                                    "Start at boot enabled."
                                }
                                .into(),
                                Err(error) => error.to_string(),
                            };
                            refresh(&mut snapshot, &mut message).await;
                        } else {
                            message = "Install the agent service to change boot startup.".into();
                        }
                    }
                    _ => {
                        refresh(&mut snapshot, &mut message).await;
                    }
                }
                last_refresh = Instant::now();
            }
            _ => {}
        }
    }
    Ok(())
}

async fn refresh(snapshot: &mut Snapshot, message: &mut String) {
    match load().await {
        Ok(next) => *snapshot = next,
        Err(error) => *message = error.to_string(),
    }
}

fn draw(
    out: &mut Stdout,
    snapshot: &Snapshot,
    host_name: &str,
    view: &ViewState<'_>,
) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    queue!(out, Clear(ClearType::All))?;
    if width < 100 || height < 24 {
        paint(
            out,
            0,
            0,
            width,
            "MSC 2 · enlarge terminal to at least 100 × 24",
            Color::White,
        )?;
        return out.flush();
    }

    // The banner and left rail match the shell in the owner's design preview.
    for y in 0..2 {
        queue!(
            out,
            MoveTo(0, y),
            SetBackgroundColor(Color::Grey),
            Print(" ".repeat(width as usize)),
            ResetColor
        )?;
    }
    paint_on(out, 1, 0, 22, "MSC 2", Color::Black, Color::Grey)?;
    paint_on(out, 1, 1, 22, "by ctemple9", Color::Black, Color::Grey)?;
    paint_on(
        out,
        width - 53,
        0,
        51,
        "Help [?]   Settings [Shift+S]   Refresh [Shift+R]",
        Color::Black,
        Color::Grey,
    )?;
    for y in 2..height - 1 {
        paint(out, 22, y, 1, "│", Color::DarkGrey)?;
    }
    paint(out, 2, 3, 18, "CURRENT HOST", Color::DarkGrey)?;
    paint(out, 2, 5, 18, host_name, Color::White)?;
    paint(out, 2, 8, 18, "Start       soon", Color::DarkGrey)?;
    paint(out, 2, 10, 18, "Manage         m", Color::White)?;
    paint(out, 2, 12, 18, "Agents         a", Color::Green)?;
    paint(out, 2, 16, 18, "SELECTED SERVER", Color::DarkGrey)?;
    let selected_server = snapshot
        .active_server_id
        .as_deref()
        .and_then(|id| {
            snapshot
                .servers
                .as_ref()?
                .iter()
                .find(|server| server.id == id)
        })
        .map(|server| server.name.as_str())
        .unwrap_or("No server selected");
    paint(out, 2, 18, 18, selected_server, Color::White)?;

    let main_x = 24;
    let main_w = width - main_x - 2;
    let gap = 2;
    let left_w = (main_w - gap) / 2;
    let right_x = main_x + left_w + gap;
    let right_w = main_w - left_w - gap;
    paint(out, main_x, 2, main_w, "Manage Agents", Color::White)?;
    paint(
        out,
        main_x,
        3,
        main_w,
        "Manage this computer's agent. Servers and files stay here.",
        Color::DarkGrey,
    )?;

    let top_y = 5;
    let top_h = 9;
    box_outline(out, main_x, top_y, left_w, top_h)?;
    box_outline(out, right_x, top_y, right_w, top_h)?;
    paint(
        out,
        main_x + 2,
        top_y + 1,
        left_w - 4,
        "CURRENT AGENT",
        Color::DarkGrey,
    )?;
    paint(
        out,
        main_x + 2,
        top_y + 2,
        left_w - 4,
        &format!("{host_name} · This computer"),
        Color::White,
    )?;

    let state = match snapshot.service.report.state {
        ServiceState::NotInstalled => "Not installed",
        ServiceState::Stopped => "Stopped",
        ServiceState::Running => "Running",
    };
    let boot = match snapshot.service.boot_enabled {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Unavailable",
    };
    let connection = if snapshot.servers.is_some() {
        "Connected"
    } else if snapshot.service.report.state == ServiceState::Running {
        "Unavailable"
    } else {
        "Service stopped"
    };
    let count = snapshot
        .servers
        .as_ref()
        .map(|list| format!("{} configured", list.len()))
        .unwrap_or_else(|| "Unavailable".into());
    let facts = [
        ("Service", state.to_string()),
        ("Starts at boot", boot.to_string()),
        ("Connection", connection.to_string()),
        ("Address", "127.0.0.1:48001".into()),
        ("Servers", count),
    ];
    for (index, (label, value)) in facts.iter().enumerate() {
        let y = top_y + 3 + index as u16;
        paint(out, main_x + 2, y, 16, label, Color::DarkGrey)?;
        paint(out, main_x + 18, y, left_w - 20, value, Color::White)?;
    }

    paint(
        out,
        right_x + 2,
        top_y + 1,
        right_w - 4,
        "ACTIONS",
        Color::DarkGrey,
    )?;
    let actions = [
        ("Use this agent", "Enter", false),
        ("Manage servers", "m", snapshot.servers.is_some()),
        (
            if snapshot.service.report.state == ServiceState::Running {
                "Stop agent service"
            } else {
                "Start agent service"
            },
            "s",
            true,
        ),
        (
            if snapshot.service.boot_enabled == Some(true) {
                "Disable start at boot"
            } else {
                "Enable start at boot"
            },
            "e",
            true,
        ),
        ("Refresh agent status", "r", true),
    ];
    for (index, (label, shortcut, ready)) in actions.iter().enumerate() {
        let y = top_y + 2 + index as u16;
        let color = if view.selected == index {
            Color::Green
        } else if *ready {
            Color::White
        } else {
            Color::DarkGrey
        };
        paint(
            out,
            right_x + 2,
            y,
            2,
            if view.selected == index { ">" } else { " " },
            color,
        )?;
        paint(out, right_x + 4, y, right_w - 16, label, color)?;
        paint(
            out,
            right_x + right_w - 11,
            y,
            9,
            if *ready { shortcut } else { "soon" },
            Color::DarkGrey,
        )?;
    }

    let bottom_y = 15;
    let bottom_h = height - bottom_y - 3;
    box_outline(out, main_x, bottom_y, left_w, bottom_h)?;
    box_outline(out, right_x, bottom_y, right_w, bottom_h)?;
    paint(
        out,
        main_x + 2,
        bottom_y + 1,
        left_w - 4,
        "ON THIS AGENT",
        Color::DarkGrey,
    )?;
    let visible = (bottom_h - 3) as usize;
    if let Some(servers) = &snapshot.servers {
        if servers.is_empty() {
            paint(
                out,
                main_x + 2,
                bottom_y + 2,
                left_w - 4,
                "No servers configured",
                Color::DarkGrey,
            )?;
        } else {
            let shown = if servers.len() > visible {
                visible - 1
            } else {
                servers.len()
            };
            for (index, server) in servers.iter().take(shown).enumerate() {
                let y = bottom_y + 2 + index as u16;
                let kind = if server.server_type.eq_ignore_ascii_case("java") {
                    format!(
                        "Java · {}",
                        server.java_flavor.as_deref().unwrap_or("Server")
                    )
                } else {
                    "Bedrock".into()
                };
                paint(
                    out,
                    main_x + 2,
                    y,
                    left_w / 2 - 2,
                    &server.name,
                    Color::White,
                )?;
                paint(
                    out,
                    main_x + left_w / 2,
                    y,
                    left_w / 2 - 2,
                    &kind,
                    Color::DarkGrey,
                )?;
            }
            if servers.len() > shown {
                paint(
                    out,
                    main_x + 2,
                    bottom_y + 2 + shown as u16,
                    left_w - 4,
                    &format!("+{} more servers", servers.len() - shown),
                    Color::DarkGrey,
                )?;
            }
        }
    } else {
        paint(
            out,
            main_x + 2,
            bottom_y + 2,
            left_w - 4,
            "Server list unavailable",
            Color::DarkGrey,
        )?;
    }
    paint(
        out,
        right_x + 2,
        bottom_y + 1,
        right_w - 4,
        "ANOTHER COMPUTER",
        Color::DarkGrey,
    )?;
    paint(
        out,
        right_x + 2,
        bottom_y + 2,
        right_w - 4,
        "SSH in as the account that installed MSC.",
        Color::White,
    )?;
    paint(
        out,
        right_x + 2,
        bottom_y + 3,
        right_w - 4,
        "Run msc tui there to control its agent.",
        Color::White,
    )?;

    let notice = if view.confirm_stop {
        "Stop the agent service? Enter confirms · Esc cancels"
    } else if !view.message.is_empty() {
        view.message
    } else {
        snapshot
            .connection_error
            .as_deref()
            .unwrap_or("Closing this screen leaves the agent and servers running.")
    };
    paint(out, main_x, height - 2, main_w, notice, Color::DarkGrey)?;
    paint(
        out,
        main_x,
        height - 1,
        main_w,
        "↑↓ Move   Enter Select   s/e/r Action   m Servers   Esc/q Quit",
        Color::DarkGrey,
    )?;
    if view.manage_open {
        draw_manage(
            out,
            snapshot,
            host_name,
            view.manage_selected,
            width,
            height,
        )?;
    }
    if let Some(wizard) = view.wizard {
        wizard.draw(out, width, height)?;
    }
    out.flush()
}

fn draw_manage(
    out: &mut Stdout,
    snapshot: &Snapshot,
    host_name: &str,
    selected: usize,
    width: u16,
    height: u16,
) -> io::Result<()> {
    let dialog_w = (width - 20).min(100);
    let dialog_h = (height - 6).min(20);
    let x = (width - dialog_w) / 2;
    let y = (height - dialog_h) / 2;
    for row in y..y + dialog_h {
        queue!(
            out,
            MoveTo(x, row),
            SetBackgroundColor(Color::Black),
            Print(" ".repeat(dialog_w as usize)),
            ResetColor
        )?;
    }
    box_outline(out, x, y, dialog_w, dialog_h)?;
    paint(
        out,
        x + 2,
        y + 1,
        dialog_w - 20,
        "Manage Servers",
        Color::White,
    )?;
    paint(
        out,
        x + dialog_w - 13,
        y + 1,
        11,
        "Esc Close",
        Color::DarkGrey,
    )?;
    let servers = snapshot.servers.as_deref().unwrap_or(&[]);
    paint(
        out,
        x + 2,
        y + 3,
        dialog_w - 4,
        &format!(
            "{} server{} configured",
            servers.len(),
            if servers.len() == 1 { "" } else { "s" }
        ),
        Color::DarkGrey,
    )?;
    paint(out, x + 2, y + 5, 20, "LOCAL AGENT", Color::DarkGrey)?;
    paint(out, x + 24, y + 5, dialog_w - 28, host_name, Color::White)?;
    let visible = (dialog_h - 10) as usize;
    if servers.is_empty() {
        paint(
            out,
            x + 3,
            y + 7,
            dialog_w - 6,
            "No servers yet · press c to create one",
            Color::DarkGrey,
        )?;
    } else {
        let start = selected.saturating_sub(visible - 1);
        for (row, server) in servers.iter().skip(start).take(visible).enumerate() {
            let index = start + row;
            let line_y = y + 7 + row as u16;
            let active = snapshot.active_server_id.as_deref() == Some(server.id.as_str());
            let kind = if server.server_type.eq_ignore_ascii_case("java") {
                format!(
                    "Java · {}",
                    server.java_flavor.as_deref().unwrap_or("Server")
                )
            } else {
                "Bedrock".into()
            };
            let status = if active && snapshot.active_server_running {
                " · Running"
            } else if active {
                " · Active"
            } else {
                ""
            };
            paint(
                out,
                x + 3,
                line_y,
                2,
                if index == selected { ">" } else { " " },
                if index == selected {
                    Color::Green
                } else {
                    Color::DarkGrey
                },
            )?;
            paint(
                out,
                x + 5,
                line_y,
                2,
                if active { "*" } else { " " },
                Color::Green,
            )?;
            paint(
                out,
                x + 7,
                line_y,
                25,
                &server.name,
                if index == selected {
                    Color::White
                } else {
                    Color::DarkGrey
                },
            )?;
            paint(
                out,
                x + 33,
                line_y,
                dialog_w - 36,
                &format!("{kind}{status}"),
                Color::DarkGrey,
            )?;
        }
        if servers.len() > visible {
            paint(
                out,
                x + dialog_w - 18,
                y + 5,
                15,
                &format!(
                    "{}–{} of {}",
                    start + 1,
                    (start + visible).min(servers.len()),
                    servers.len()
                ),
                Color::DarkGrey,
            )?;
        }
    }
    paint(
        out,
        x + 2,
        y + dialog_h - 2,
        dialog_w - 4,
        "t Transfer soon   c Create server   ↑↓ Move   Enter Select   Esc Close",
        Color::DarkGrey,
    )
}

fn box_outline(out: &mut Stdout, x: u16, y: u16, width: u16, height: u16) -> io::Result<()> {
    let rule = "─".repeat((width - 2) as usize);
    paint(out, x, y, width, &format!("┌{rule}┐"), Color::DarkGrey)?;
    for row in y + 1..y + height - 1 {
        paint(out, x, row, 1, "│", Color::DarkGrey)?;
        paint(out, x + width - 1, row, 1, "│", Color::DarkGrey)?;
    }
    paint(
        out,
        x,
        y + height - 1,
        width,
        &format!("└{rule}┘"),
        Color::DarkGrey,
    )
}

fn paint(
    out: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    value: &str,
    color: Color,
) -> io::Result<()> {
    let clipped: String = value.chars().take(width as usize).collect();
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(color),
        Print(clipped),
        ResetColor
    )
}

fn paint_on(
    out: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    value: &str,
    foreground: Color,
    background: Color,
) -> io::Result<()> {
    let clipped: String = value.chars().take(width as usize).collect();
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(clipped),
        ResetColor
    )
}
