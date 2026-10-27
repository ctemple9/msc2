//! Host-local terminal client. The first screen owns only presentation and
//! delegates service changes and server reads to the existing CLI paths.

use std::io::{self, IsTerminal, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use msc_api::dto::ServerDto;
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
    connection_error: Option<String>,
}

async fn load() -> Result<Snapshot, CliError> {
    let service = service::local_agent_status()?;
    let (servers, connection_error) = if service.report.state == ServiceState::Running {
        match SharedClient::connect_local().await {
            Ok(client) => match client.get_json::<Vec<ServerDto>>("/v1/servers").await {
                Ok(servers) => (Some(servers), None),
                Err(error) => (None, Some(error.to_string())),
            },
            Err(error) => (None, Some(error.to_string())),
        }
    } else {
        (None, None)
    };
    Ok(Snapshot {
        service,
        servers,
        connection_error,
    })
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
    let mut message = String::new();
    let mut confirm_stop = false;
    let mut last_refresh = Instant::now();

    loop {
        draw(
            &mut stdout,
            &snapshot,
            &host_name,
            selected,
            &message,
            confirm_stop,
        )
        .map_err(terminal_error)?;
        if !event::poll(Duration::from_millis(250)).map_err(terminal_error)? {
            if last_refresh.elapsed() >= REFRESH_INTERVAL {
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
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => break,
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down => selected = (selected + 1).min(2),
            KeyCode::Char('s') => selected = 0,
            KeyCode::Char('e') => selected = 1,
            KeyCode::Char('r') | KeyCode::Char('R') => selected = 2,
            KeyCode::Enter => {
                match selected {
                    0 => {
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
                    1 => {
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
    selected: usize,
    message: &str,
    confirm_stop: bool,
) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    queue!(out, Clear(ClearType::All))?;
    if width < 45 || height < 22 {
        line(
            out,
            0,
            0,
            width,
            "MSC 2 · terminal too small (need 45 × 22)",
            Color::White,
        )?;
        return out.flush();
    }
    queue!(
        out,
        MoveTo(0, 0),
        SetBackgroundColor(Color::Grey),
        SetForegroundColor(Color::Black),
        Print(format!(
            "{:<width$}",
            " MSC 2  by ctemple9",
            width = width as usize
        )),
        ResetColor
    )?;
    line(out, 2, 2, width - 4, "MANAGE AGENTS", Color::White)?;
    line(
        out,
        2,
        3,
        width - 4,
        &format!("{host_name} · this computer"),
        Color::DarkGrey,
    )?;
    section(out, 2, 5, width - 4, "CURRENT AGENT")?;
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
    line(
        out,
        4,
        7,
        width - 8,
        &format!("Service          {state}"),
        Color::White,
    )?;
    line(
        out,
        4,
        8,
        width - 8,
        &format!("Starts at boot   {boot}"),
        Color::White,
    )?;
    line(
        out,
        4,
        9,
        width - 8,
        &format!("Connection       {connection}"),
        Color::White,
    )?;
    line(
        out,
        4,
        10,
        width - 8,
        "Address          127.0.0.1:48001",
        Color::White,
    )?;
    line(
        out,
        4,
        11,
        width - 8,
        &format!("Servers          {count}"),
        Color::White,
    )?;
    section(out, 2, 13, width - 4, "ACTIONS")?;
    let actions = [
        if snapshot.service.report.state == ServiceState::Running {
            "Stop agent service"
        } else {
            "Start agent service"
        },
        if snapshot.service.boot_enabled == Some(true) {
            "Disable start at boot"
        } else {
            "Enable start at boot"
        },
        "Refresh agent status",
    ];
    for (index, action) in actions.iter().enumerate() {
        line(
            out,
            4,
            15 + index as u16,
            width - 8,
            &format!(
                "{} {action}  [{}]",
                if selected == index { '>' } else { ' ' },
                ["s", "e", "r"][index]
            ),
            if selected == index {
                Color::Green
            } else {
                Color::White
            },
        )?;
    }
    if height >= 27 {
        section(out, 2, 19, width - 4, "ON THIS AGENT")?;
        if let Some(servers) = &snapshot.servers {
            if servers.is_empty() {
                line(
                    out,
                    4,
                    21,
                    width - 8,
                    "No servers configured",
                    Color::DarkGrey,
                )?;
            }
            for (index, server) in servers.iter().take((height - 26) as usize).enumerate() {
                line(
                    out,
                    4,
                    21 + index as u16,
                    width - 8,
                    &format!("{}  ·  {}", server.name, server.server_type),
                    Color::White,
                )?;
            }
        } else {
            line(
                out,
                4,
                21,
                width - 8,
                "Server list unavailable until the agent API is connected",
                Color::DarkGrey,
            )?;
        }
    }
    let notice = if confirm_stop {
        "Stop the agent service? Enter confirm · Esc cancel"
    } else if !message.is_empty() {
        message
    } else {
        snapshot
            .connection_error
            .as_deref()
            .unwrap_or("Closing this screen leaves the agent and servers running.")
    };
    line(out, 2, height - 3, width - 4, notice, Color::DarkGrey)?;
    line(
        out,
        2,
        height - 1,
        width - 4,
        "↑↓ Move   Enter Select   s/e/r Choose action   Esc/q Quit",
        Color::DarkGrey,
    )?;
    out.flush()
}

fn section(out: &mut Stdout, x: u16, y: u16, width: u16, title: &str) -> io::Result<()> {
    line(
        out,
        x,
        y,
        width,
        &format!(
            "{title} {}",
            "─".repeat(width.saturating_sub(title.len() as u16 + 2) as usize)
        ),
        Color::DarkGrey,
    )
}

fn line(out: &mut Stdout, x: u16, y: u16, width: u16, value: &str, color: Color) -> io::Result<()> {
    let text: String = value.chars().take(width as usize).collect();
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(color),
        Print(text),
        ResetColor
    )
}
