//! Host-local terminal client. Screens delegate service changes and server
//! reads to the existing CLI paths.

mod components;
mod create;
mod files;
mod initiate;
mod performance;
mod server_editor;
mod settings;
mod world_settings;
mod worlds;

use std::io::{self, IsTerminal, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use msc_api::dto::{
    ActiveServerRequestDto, AddonsResponseDto, BackupConfigResponseDto, BackupsResponseDto,
    BroadcastJarStatusDto, ComponentsStatusDto, PerformanceSnapshotDto, RemoteApiStatus, ServerDto,
    SettingsResponseDto, SimpleResultDto, WorldSlotsResponseDto,
};
use msc_infrastructure::console_buffer::ConsoleLine;
use msc_infrastructure::service::ServiceState;

use super::service::{self, AgentAction, LocalAgentStatus};
use super::transport::SharedClient;
use super::{CliError, CommonArgs};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const SERVER_TABS: [&str; 7] = [
    "Overview",
    "Players",
    "Worlds",
    "Performance",
    "Components",
    "Settings",
    "Files",
];

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
    performance: Option<PerformanceSnapshotDto>,
    online_players: Option<u64>,
    online_names: Option<Vec<String>>,
    known_players: Option<Vec<String>>,
    active_world: Option<String>,
    worlds: Option<WorldSlotsResponseDto>,
    backups: Option<BackupsResponseDto>,
    backup_config: Option<BackupConfigResponseDto>,
    components: Option<ComponentsStatusDto>,
    addons: Option<AddonsResponseDto>,
    broadcast_jar: Option<BroadcastJarStatusDto>,
    settings: Option<SettingsResponseDto>,
    console_lines: Option<Vec<ConsoleLine>>,
}

struct ViewState<'a> {
    selected: usize,
    message: &'a str,
    confirm_stop: bool,
    manage_open: bool,
    manage_selected: usize,
    workspace_open: bool,
    tab_index: usize,
    worlds: &'a worlds::WorldsUi,
    performance: &'a performance::PerformanceUi,
    components: &'a components::ComponentsUi,
    settings: &'a settings::SettingsUi,
    files: &'a files::FilesUi,
    wizard: Option<&'a create::Wizard>,
    server_editor: Option<&'a server_editor::ServerEditor>,
    initiate: Option<&'a initiate::InitiateUi>,
    initiate_hidden: bool,
    console_expanded: bool,
}

async fn load() -> Result<Snapshot, CliError> {
    let service = service::local_agent_status()?;
    let (servers, active_server_id, active_server_running, connection_error) =
        match SharedClient::connect_local().await {
            Ok(client) => match client.get_json::<Vec<ServerDto>>("/v1/servers").await {
                Ok(servers) => match client.get_json::<RemoteApiStatus>("/v1/status").await {
                    Ok(status) => (Some(servers), status.active_server_id, status.running, None),
                    Err(error) => (Some(servers), None, false, Some(error.to_string())),
                },
                Err(error) => (None, None, false, Some(error.to_string())),
            },
            Err(error) => (None, None, false, Some(error.to_string())),
        };
    let (
        performance,
        online_players,
        online_names,
        known_players,
        active_world,
        worlds,
        backups,
        backup_config,
        components,
        addons,
        broadcast_jar,
        settings,
    ) = if active_server_id.is_some() {
        match SharedClient::connect_local().await {
            Ok(client) => {
                let (
                    performance,
                    players,
                    profiles,
                    worlds,
                    backups,
                    backup_config,
                    components,
                    addons,
                    broadcast_jar,
                    settings,
                ) = tokio::join!(
                    client.get_json("/v1/performance"),
                    client.get_json::<serde_json::Value>("/v1/players"),
                    client.get_json::<serde_json::Value>("/v1/players/profiles"),
                    client.get_json::<WorldSlotsResponseDto>("/v1/worlds"),
                    client.get_json::<BackupsResponseDto>("/v1/backups"),
                    client.get_json::<BackupConfigResponseDto>("/v1/backups/config"),
                    client.get_json::<ComponentsStatusDto>("/v1/components"),
                    client.get_json::<AddonsResponseDto>("/v1/addons?local=true"),
                    client.get_json::<BroadcastJarStatusDto>("/v1/broadcast/jar-status"),
                    client.get_json::<SettingsResponseDto>("/v1/settings"),
                );
                let players = players.ok();
                let online_players = players.as_ref().and_then(|value| value["count"].as_u64());
                let online_names = players
                    .as_ref()
                    .and_then(|value| value["players"].as_array())
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
                            .collect()
                    });
                let known_players = profiles.ok().and_then(|value| {
                    value["profiles"].as_array().map(|entries| {
                        entries
                            .iter()
                            .filter_map(|entry| entry["username"].as_str().map(str::to_owned))
                            .collect()
                    })
                });
                let worlds = worlds.ok();
                let active_world = worlds.as_ref().and_then(|worlds| {
                    worlds
                        .slots
                        .iter()
                        .find(|slot| slot.is_active)
                        .map(|slot| slot.name.clone())
                });
                (
                    performance.ok(),
                    online_players,
                    online_names,
                    known_players,
                    active_world,
                    worlds,
                    backups.ok(),
                    backup_config.ok(),
                    components.ok(),
                    addons.ok(),
                    broadcast_jar.ok(),
                    settings.ok(),
                )
            }
            Err(_) => (
                None, None, None, None, None, None, None, None, None, None, None, None,
            ),
        }
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None, None,
        )
    };
    let console_lines = if active_server_id.is_some() {
        if let Ok(client) = SharedClient::connect_local().await {
            client
                .get_json::<Vec<ConsoleLine>>("/v1/console/tail?n=40")
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };
    Ok(Snapshot {
        service,
        servers,
        active_server_id,
        active_server_running,
        connection_error,
        performance,
        online_players,
        online_names,
        known_players,
        active_world,
        worlds,
        backups,
        backup_config,
        components,
        addons,
        broadcast_jar,
        settings,
        console_lines,
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

async fn server_lifecycle_action(path: &str) -> Result<SimpleResultDto, CliError> {
    let client = SharedClient::connect_local().await?;
    client.post_json(path, &serde_json::json!({})).await
}

async fn selected_server_action(
    snapshot: &mut Snapshot,
    message: &mut String,
) -> Option<initiate::InitiateUi> {
    refresh(snapshot, message).await;
    if let Some(error) = &snapshot.connection_error {
        *message = format!("Cannot control the server while the agent is unavailable: {error}");
        return None;
    }
    let server = snapshot.active_server_id.as_deref().and_then(|id| {
        snapshot
            .servers
            .as_ref()?
            .iter()
            .find(|server| server.id == id)
    });
    let Some(server) = server else {
        *message = "Select a server in Manage first.".into();
        return None;
    };
    if snapshot.active_server_running {
        *message = match server_lifecycle_action("/v1/stop").await {
            Ok(_) => "Stop requested. Waiting for the server to shut down.".into(),
            Err(error) => error.to_string(),
        };
    } else if server.first_start_required == Some(true) {
        return Some(initiate::InitiateUi::new(server.clone()));
    } else {
        *message = match server_lifecycle_action("/v1/start").await {
            Ok(_) => "Start requested. Watch the server status and console.".into(),
            Err(error) => error.to_string(),
        };
    }
    refresh(snapshot, message).await;
    None
}

async fn discard_hidden_initiation(server_id: &str) -> Result<(), String> {
    let client = SharedClient::connect_local()
        .await
        .map_err(|e| e.to_string())?;
    let status: RemoteApiStatus = client
        .get_json("/v1/status")
        .await
        .map_err(|e| e.to_string())?;
    if status.running {
        if status.active_server_id.as_deref() != Some(server_id) {
            return Err("Select the server with the hidden setup before discarding it.".into());
        }
        server_lifecycle_action("/v1/stop")
            .await
            .map_err(|e| e.to_string())?;
        for _ in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let current: RemoteApiStatus = client
                .get_json("/v1/status")
                .await
                .map_err(|e| e.to_string())?;
            if !current.running {
                break;
            }
        }
    }
    let current: RemoteApiStatus = client
        .get_json("/v1/status")
        .await
        .map_err(|e| e.to_string())?;
    if current.running {
        return Err("The server is still stopping. Try again when it stops.".into());
    }
    let _: SimpleResultDto = client
        .post_json(
            "/v1/first-start/discard",
            &serde_json::json!({"serverId": server_id}),
        )
        .await
        .map_err(|e| e.to_string())?;
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
    let mut workspace_open = false;
    let mut tab_index = 0usize;
    let mut worlds_ui = worlds::WorldsUi::default();
    let mut performance_ui = performance::PerformanceUi::default();
    let mut components_ui = components::ComponentsUi::default();
    let mut settings_ui = settings::SettingsUi::default();
    let mut files_ui = files::FilesUi::default();
    let mut wizard: Option<create::Wizard> = None;
    let mut server_editor: Option<server_editor::ServerEditor> = None;
    let mut initiate: Option<initiate::InitiateUi> = None;
    let mut initiate_hidden = false;
    let mut console_expanded = false;
    let mut message = String::new();
    let mut confirm_stop = false;
    let mut last_refresh = Instant::now();
    let mut last_initiate_poll = Instant::now();

    loop {
        performance_ui.record(&snapshot);
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
                workspace_open,
                tab_index,
                worlds: &worlds_ui,
                performance: &performance_ui,
                components: &components_ui,
                settings: &settings_ui,
                files: &files_ui,
                wizard: wizard.as_ref(),
                server_editor: server_editor.as_ref(),
                initiate: initiate.as_ref(),
                initiate_hidden,
                console_expanded,
            },
        )
        .map_err(terminal_error)?;
        if !event::poll(Duration::from_millis(250)).map_err(terminal_error)? {
            if let Some(flow) = &mut initiate
                && last_initiate_poll.elapsed() >= Duration::from_secs(1)
                && snapshot
                    .active_server_id
                    .as_deref()
                    .is_none_or(|id| id == flow.server_id())
            {
                flow.poll().await;
                last_initiate_poll = Instant::now();
            }
            if !manage_open && wizard.is_none() && last_refresh.elapsed() >= REFRESH_INTERVAL {
                worlds_ui.poll_operation(&mut message).await;
                components_ui.poll_operation(&mut message).await;
                if let Some(editor) = &mut server_editor {
                    editor.poll_operation(&mut message).await;
                    editor.poll_services(&mut message).await;
                }
                refresh(&mut snapshot, &mut message).await;
                worlds_ui.refreshed();
                components_ui.refreshed();
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
                    workspace_open = true;
                    tab_index = 0;
                }
            }
            continue;
        }
        if let Some(editor) = &mut server_editor {
            if editor.handle(key.code, &mut message).await {
                server_editor = None;
            } else if editor.needs_refresh() {
                refresh(&mut snapshot, &mut message).await;
                editor.refreshed();
            }
            continue;
        }
        if let Some(flow) = &mut initiate
            && !initiate_hidden
        {
            let close = flow.handle(key.code).await;
            if flow.take_hide_request() {
                initiate_hidden = true;
                message = "Initiation hidden. Press i to resume or d to discard setup.".into();
            }
            if let Some(broadcast) = flow.take_service_request() {
                server_editor = Some(
                    server_editor::ServerEditor::open_service(flow.server().clone(), broadcast)
                        .await,
                );
            }
            if close {
                initiate = None;
                initiate_hidden = false;
                refresh(&mut snapshot, &mut message).await;
            }
            continue;
        }
        if initiate_hidden
            && initiate
                .as_ref()
                .is_some_and(|flow| snapshot.active_server_id.as_deref() == Some(flow.server_id()))
        {
            match key.code {
                KeyCode::Char('i') => {
                    initiate_hidden = false;
                    workspace_open = true;
                    continue;
                }
                KeyCode::Char('d') => {
                    let id = initiate
                        .as_ref()
                        .expect("hidden initiation exists")
                        .server_id()
                        .to_string();
                    match discard_hidden_initiation(&id).await {
                        Ok(()) => {
                            initiate = None;
                            initiate_hidden = false;
                            refresh(&mut snapshot, &mut message).await;
                            message =
                                "Setup progress discarded. The existing world was kept.".into();
                        }
                        Err(error) => message = format!("Could not discard setup: {error}"),
                    }
                    continue;
                }
                _ => {}
            }
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
                                workspace_open = true;
                                tab_index = 0;
                                refresh(&mut snapshot, &mut message).await;
                                message = format!("Selected server: {name}");
                                if let Some(server) =
                                    snapshot.servers.as_ref().and_then(|servers| {
                                        servers.iter().find(|server| server.id == id)
                                    })
                                {
                                    server_editor = Some(
                                        server_editor::ServerEditor::open(server.clone()).await,
                                    );
                                }
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
        if workspace_open {
            if key.code == KeyCode::Char('c') {
                console_expanded = !console_expanded;
                continue;
            }
            if tab_index == 6
                && files_ui
                    .handle(key.code, &snapshot, &mut stdout, &mut message)
                    .await
            {
                continue;
            }
            if tab_index == 5 && settings_ui.handle(key.code, &snapshot, &mut message).await {
                if settings_ui.needs_refresh() {
                    refresh(&mut snapshot, &mut message).await;
                    settings_ui.refreshed();
                }
                continue;
            }
            if tab_index == 4
                && components_ui
                    .handle(key.code, &snapshot, &mut message)
                    .await
            {
                if components_ui.needs_refresh() {
                    refresh(&mut snapshot, &mut message).await;
                    components_ui.refreshed();
                }
                continue;
            }
            if tab_index == 2 && worlds_ui.handle(key.code, &snapshot, &mut message).await {
                if worlds_ui.needs_refresh() {
                    refresh(&mut snapshot, &mut message).await;
                    worlds_ui.refreshed();
                }
                continue;
            }
            if key.code == KeyCode::Char('s') {
                if initiate_hidden
                    && !snapshot.active_server_running
                    && initiate.as_ref().is_some_and(|flow| {
                        snapshot.active_server_id.as_deref() == Some(flow.server_id())
                    })
                {
                    message = "Setup is hidden. Press i to resume or d to discard it.".into();
                    continue;
                }
                if let Some(flow) = selected_server_action(&mut snapshot, &mut message).await {
                    initiate = Some(flow);
                    initiate_hidden = false;
                }
                continue;
            }
            match key.code {
                KeyCode::Char('1'..='7') => {
                    if let KeyCode::Char(digit) = key.code {
                        tab_index = digit as usize - '1' as usize;
                        if tab_index == 6 {
                            files_ui.open(&snapshot, &mut message).await;
                        }
                    }
                }
                KeyCode::Char('a') | KeyCode::Esc => workspace_open = false,
                KeyCode::Char('m') => {
                    if snapshot.servers.is_some() {
                        manage_open = true;
                    } else {
                        message = "Agent API unavailable; refresh its connection.".into();
                    }
                }
                KeyCode::Char('R') => {
                    refresh(&mut snapshot, &mut message).await;
                    last_refresh = Instant::now();
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
            KeyCode::Char('s') => {
                if initiate_hidden
                    && !snapshot.active_server_running
                    && initiate.as_ref().is_some_and(|flow| {
                        snapshot.active_server_id.as_deref() == Some(flow.server_id())
                    })
                {
                    message = "Setup is hidden. Press i to resume or d to discard it.".into();
                    continue;
                }
                if let Some(flow) = selected_server_action(&mut snapshot, &mut message).await {
                    initiate = Some(flow);
                    initiate_hidden = false;
                    workspace_open = true;
                }
            }
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
                        workspace_open = true;
                        tab_index = 0;
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
        Ok(mut next) => {
            if next.connection_error.is_some() || next.servers.is_none() {
                // A failed API read must not erase the selected server from
                // the screen or make an open initiation lose its server context.
                if next.servers.is_none() {
                    next.servers = snapshot.servers.take();
                }
                next.active_server_id = snapshot.active_server_id.take();
                next.active_server_running = snapshot.active_server_running;
                if next.connection_error.is_none() {
                    next.connection_error = Some("Agent service is unavailable.".into());
                }
            }
            *snapshot = next;
        }
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
    if view.workspace_open {
        draw_workspace(
            out,
            snapshot,
            view.tab_index,
            view.worlds,
            view.performance,
            view.components,
            view.settings,
            view.files,
            view.message,
            view.initiate.filter(|_| view.initiate_hidden),
            view.console_expanded,
            width,
            height,
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
        if let Some(editor) = view.server_editor {
            editor.draw(out, width, height)?;
        } else if let Some(initiate) = view.initiate.filter(|_| !view.initiate_hidden) {
            initiate.draw(out, width, height)?;
        }
        return out.flush();
    }
    for y in 2..height - 1 {
        paint(out, 22, y, 1, "│", Color::DarkGrey)?;
    }
    paint(out, 2, 3, 18, "CURRENT HOST", Color::DarkGrey)?;
    paint(out, 2, 5, 18, host_name, Color::White)?;
    let server = snapshot.active_server_id.as_deref().and_then(|id| {
        snapshot
            .servers
            .as_ref()?
            .iter()
            .find(|server| server.id == id)
    });
    let action = if snapshot.active_server_running {
        "Stop           s"
    } else if server.is_some_and(|server| server.first_start_required == Some(true)) {
        "Initiate       s"
    } else {
        "Start          s"
    };
    paint(
        out,
        2,
        8,
        18,
        action,
        if server.is_some() && snapshot.connection_error.is_none() {
            Color::White
        } else {
            Color::DarkGrey
        },
    )?;
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
    if view.initiate_hidden
        && view
            .initiate
            .is_some_and(|flow| snapshot.active_server_id.as_deref() == Some(flow.server_id()))
    {
        paint(out, 2, 20, 18, "Resume init.   i", Color::Green)?;
        paint(out, 2, 21, 18, "Discard setup  d", Color::White)?;
    }

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
    let connection = if snapshot.connection_error.is_some() {
        "Reconnecting"
    } else if snapshot.servers.is_some() {
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
        ("Use this agent", "Enter", true),
        ("Manage servers", "m", snapshot.servers.is_some()),
        (
            if snapshot.service.report.state == ServiceState::Running {
                "Stop agent service"
            } else {
                "Start agent service"
            },
            "Enter",
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
        "↑↓ Move   Enter Select   s Server   m Servers   e/r Agent action   Esc/q Quit",
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

fn draw_workspace(
    out: &mut Stdout,
    snapshot: &Snapshot,
    tab_index: usize,
    worlds_ui: &worlds::WorldsUi,
    performance_ui: &performance::PerformanceUi,
    components_ui: &components::ComponentsUi,
    settings_ui: &settings::SettingsUi,
    files_ui: &files::FilesUi,
    message: &str,
    hidden_initiate: Option<&initiate::InitiateUi>,
    console_expanded: bool,
    width: u16,
    height: u16,
) -> io::Result<()> {
    let server = snapshot.active_server_id.as_deref().and_then(|id| {
        snapshot
            .servers
            .as_ref()?
            .iter()
            .find(|server| server.id == id)
    });
    for y in 2..height - 1 {
        paint(out, 22, y, 1, "│", Color::DarkGrey)?;
    }
    paint(out, 2, 3, 18, "SELECTED SERVER", Color::DarkGrey)?;
    paint(
        out,
        2,
        5,
        18,
        server.map_or("No server selected", |server| server.name.as_str()),
        Color::White,
    )?;
    let action = if snapshot.active_server_running {
        "Stop           s"
    } else if server.is_some_and(|server| server.first_start_required == Some(true)) {
        "Initiate       s"
    } else {
        "Start          s"
    };
    paint(
        out,
        2,
        8,
        18,
        action,
        if server.is_some() && snapshot.connection_error.is_none() {
            Color::White
        } else {
            Color::DarkGrey
        },
    )?;
    paint(out, 2, 10, 18, "Manage         m", Color::White)?;
    paint(out, 2, 12, 18, "Agents         a", Color::White)?;
    if hidden_initiate
        .is_some_and(|flow| snapshot.active_server_id.as_deref() == Some(flow.server_id()))
    {
        paint(out, 2, 16, 18, "Resume init.   i", Color::Green)?;
        paint(out, 2, 17, 18, "Discard setup  d", Color::White)?;
    }

    let main_x = 24;
    let main_w = width - main_x - 2;
    let mut tab_x = main_x;
    for (index, label) in SERVER_TABS.iter().enumerate() {
        let label = if width < 120 {
            match index {
                3 => "Perf",
                4 => "Comps",
                _ => label,
            }
        } else {
            label
        };
        let tab = format!("{} {}", index + 1, label);
        let tab_w = tab.chars().count() as u16 + 2;
        if tab_x + tab_w > width - 2 {
            break;
        }
        paint(
            out,
            tab_x,
            3,
            tab_w,
            &tab,
            if index == tab_index {
                Color::Green
            } else {
                Color::DarkGrey
            },
        )?;
        tab_x += tab_w;
    }
    paint(
        out,
        main_x,
        4,
        main_w,
        &"─".repeat(main_w as usize),
        Color::DarkGrey,
    )?;

    if tab_index == 0 && height >= 40 {
        paint(out, main_x, 6, main_w, "STATUS", Color::DarkGrey)?;
        let gap = 2;
        let left_w = (main_w - gap) / 2;
        let right_x = main_x + left_w + gap;
        let right_w = main_w - left_w - gap;
        box_outline(out, main_x, 8, left_w, if server.is_some() { 9 } else { 6 })?;
        box_outline(
            out,
            right_x,
            8,
            right_w,
            if server.is_some() { 9 } else { 6 },
        )?;
        paint(
            out,
            main_x + 2,
            9,
            left_w - 4,
            "Connection Info",
            Color::White,
        )?;
        paint(out, right_x + 2, 9, right_w - 4, "Live Stats", Color::White)?;
        if let Some(server) = server {
            let software = if server.server_type.eq_ignore_ascii_case("java") {
                format!(
                    "Java · {}",
                    server.java_flavor.as_deref().unwrap_or("Server")
                )
            } else {
                "Bedrock".into()
            };
            let port = server.game_port.or(server.bedrock_port);
            paint(
                out,
                main_x + 2,
                11,
                left_w - 4,
                &format!("Software  {software}"),
                Color::White,
            )?;
            paint(
                out,
                main_x + 2,
                12,
                left_w - 4,
                &format!(
                    "Local     localhost:{}",
                    port.map_or_else(|| "—".into(), |p| p.to_string())
                ),
                Color::White,
            )?;
            paint(
                out,
                main_x + 2,
                13,
                left_w - 4,
                &format!(
                    "Remote    {}",
                    server.host_address.as_deref().unwrap_or("Not configured")
                ),
                Color::White,
            )?;
            paint(
                out,
                right_x + 2,
                11,
                right_w - 4,
                &format!(
                    "State     {}",
                    if snapshot.active_server_running {
                        "Running"
                    } else {
                        "Stopped"
                    }
                ),
                Color::White,
            )?;
            let metric = snapshot.performance.as_ref();
            let tps = metric
                .and_then(|value| value.tps_1m.as_ref())
                .map(|value| format!("{:.1}", value.value))
                .unwrap_or_else(|| "—".into());
            let cpu = metric
                .and_then(|value| value.cpu_percent.as_ref())
                .map(|value| format!("{:.0}%", value.value))
                .unwrap_or_else(|| "—".into());
            let memory = metric
                .and_then(|value| value.ram_used_mb.as_ref())
                .map(|value| format!("{:.0} MB", value.value))
                .unwrap_or_else(|| "—".into());
            paint(
                out,
                right_x + 2,
                12,
                right_w - 4,
                &format!("TPS       {tps}"),
                Color::White,
            )?;
            paint(
                out,
                right_x + 2,
                13,
                right_w - 4,
                &format!("CPU       {cpu}"),
                Color::White,
            )?;
            paint(
                out,
                right_x + 2,
                14,
                right_w - 4,
                &format!("Memory    {memory}"),
                Color::White,
            )?;
        } else {
            paint(
                out,
                main_x + 2,
                11,
                left_w - 4,
                "Server   No server selected",
                Color::White,
            )?;
            paint(out, main_x + 2, 12, left_w - 4, "Address  —", Color::White)?;
            paint(
                out,
                right_x + 2,
                11,
                right_w - 4,
                "State    Waiting for server",
                Color::White,
            )?;
            paint(
                out,
                right_x + 2,
                12,
                right_w - 4,
                "Players  —",
                Color::White,
            )?;
        }
        let activity_y = if server.is_some() { 19 } else { 16 };
        paint(
            out,
            main_x,
            activity_y - 1,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
        paint(out, main_x, activity_y, main_w, "ACTIVITY", Color::DarkGrey)?;
        let card_y = activity_y + 2;
        let card_gap = 2;
        let card_w = (main_w - 2 * card_gap) / 3;
        let third_x = main_x + 2 * (card_w + card_gap);
        let third_w = main_w - 2 * (card_w + card_gap);
        box_outline(
            out,
            main_x,
            card_y,
            card_w,
            if server.is_some() { 6 } else { 5 },
        )?;
        box_outline(
            out,
            main_x + card_w + card_gap,
            card_y,
            card_w,
            if server.is_some() { 6 } else { 5 },
        )?;
        box_outline(
            out,
            third_x,
            card_y,
            third_w,
            if server.is_some() { 6 } else { 5 },
        )?;
        paint(
            out,
            main_x + 2,
            card_y + 1,
            card_w - 4,
            "Players",
            Color::White,
        )?;
        paint(
            out,
            main_x + card_w + card_gap + 2,
            card_y + 1,
            card_w - 4,
            "Active World",
            Color::White,
        )?;
        paint(
            out,
            third_x + 2,
            card_y + 1,
            third_w - 4,
            "Chat",
            Color::White,
        )?;
        if server.is_some() {
            let players = snapshot
                .online_players
                .map_or_else(|| "— online".into(), |count| format!("{count} online"));
            paint(
                out,
                main_x + 2,
                card_y + 3,
                card_w - 4,
                &players,
                Color::White,
            )?;
            paint(
                out,
                main_x + card_w + card_gap + 2,
                card_y + 3,
                card_w - 4,
                snapshot
                    .active_world
                    .as_deref()
                    .unwrap_or("World unavailable"),
                Color::White,
            )?;
        } else {
            paint(
                out,
                main_x + 2,
                card_y + 3,
                card_w - 4,
                "No server selected",
                Color::White,
            )?;
            paint(
                out,
                main_x + card_w + card_gap + 2,
                card_y + 3,
                card_w - 4,
                "—",
                Color::White,
            )?;
        }
        paint(
            out,
            third_x + 2,
            card_y + 3,
            third_w - 4,
            "Chat feed coming next",
            Color::DarkGrey,
        )?;
        let notes_y = card_y + if server.is_some() { 8 } else { 7 };
        paint(
            out,
            main_x,
            notes_y - 1,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
        paint(out, main_x, notes_y, main_w, "NOTES", Color::DarkGrey)?;
        paint(
            out,
            main_x + 2,
            notes_y + 2,
            main_w - 2,
            server.map_or("Select a server to view its notes.", |server| {
                if server.notes.is_empty() {
                    "For this server · No notes yet"
                } else {
                    server.notes.as_str()
                }
            }),
            Color::White,
        )?;
    } else if tab_index == 0 {
        paint(out, main_x, 6, main_w, "STATUS", Color::DarkGrey)?;
        paint(
            out,
            main_x + 2,
            8,
            main_w - 2,
            server.map_or("Server   No server selected", |server| server.name.as_str()),
            Color::White,
        )?;
        paint(
            out,
            main_x + 2,
            9,
            main_w - 2,
            if server.is_some() {
                if snapshot.active_server_running {
                    "State    Running"
                } else {
                    "State    Stopped"
                }
            } else {
                "State    Waiting for server"
            },
            Color::White,
        )?;
        paint(out, main_x, 12, main_w, "ACTIVITY", Color::DarkGrey)?;
        paint(
            out,
            main_x + 2,
            14,
            main_w - 2,
            snapshot
                .active_world
                .as_deref()
                .unwrap_or("No active world"),
            Color::White,
        )?;
        paint(out, main_x, 17, main_w, "NOTES", Color::DarkGrey)?;
        paint(
            out,
            main_x + 2,
            18,
            main_w - 2,
            server.map_or("Select a server to view its notes.", |server| {
                if server.notes.is_empty() {
                    "For this server · No notes yet"
                } else {
                    server.notes.as_str()
                }
            }),
            Color::White,
        )?;
    } else if tab_index == 1 {
        if server.is_none() {
            paint(
                out,
                main_x,
                7,
                main_w,
                "Select or create a server to view players.",
                Color::DarkGrey,
            )?;
        } else {
            let count = snapshot
                .online_players
                .map_or_else(|| "—".into(), |count| count.to_string());
            paint(
                out,
                main_x,
                6,
                main_w,
                &format!("ONLINE  {count}"),
                Color::White,
            )?;
            match snapshot.online_names.as_deref() {
                Some([]) => paint(out, main_x, 8, main_w, "No players online.", Color::White)?,
                Some(names) => {
                    for (index, name) in names
                        .iter()
                        .take(height.saturating_sub(20).min(8) as usize)
                        .enumerate()
                    {
                        paint(out, main_x, 8 + index as u16, main_w, name, Color::White)?;
                    }
                }
                None => paint(
                    out,
                    main_x,
                    8,
                    main_w,
                    "Online players unavailable.",
                    Color::DarkGrey,
                )?,
            }
            let history_y = 11
                + snapshot.online_names.as_ref().map_or(0, |names| {
                    names.len().min(height.saturating_sub(20).min(8) as usize) as u16
                });
            match snapshot.known_players.as_deref() {
                Some([]) => paint(
                    out,
                    main_x,
                    history_y,
                    main_w,
                    "Session history is empty.",
                    Color::White,
                )?,
                Some(names) => {
                    paint(
                        out,
                        main_x,
                        history_y,
                        main_w,
                        "KNOWN PLAYERS",
                        Color::DarkGrey,
                    )?;
                    for (index, name) in names
                        .iter()
                        .take(height.saturating_sub(history_y + 8) as usize)
                        .enumerate()
                    {
                        paint(
                            out,
                            main_x,
                            history_y + 2 + index as u16,
                            main_w,
                            name,
                            Color::White,
                        )?;
                    }
                }
                None => paint(
                    out,
                    main_x,
                    history_y,
                    main_w,
                    "Player history unavailable.",
                    Color::DarkGrey,
                )?,
            }
        }
    } else if tab_index == 2 {
        worlds_ui.draw(out, snapshot, main_x, main_w, height)?;
    } else if tab_index == 3 {
        performance_ui.draw(out, snapshot, main_x, main_w, height)?;
    } else if tab_index == 4 {
        components_ui.draw(out, snapshot, main_x, main_w, height)?;
    } else if tab_index == 5 {
        settings_ui.draw(out, snapshot, main_x, main_w, height)?;
    } else if tab_index == 6 {
        files_ui.draw(out, snapshot, main_x, main_w, height)?;
    } else {
        paint(
            out,
            main_x,
            7,
            main_w,
            &SERVER_TABS[tab_index].to_ascii_uppercase(),
            Color::White,
        )?;
        paint(
            out,
            main_x,
            9,
            main_w,
            if server.is_some() {
                "This section is ready for the next TUI step."
            } else {
                "Select or create a server to view this section."
            },
            Color::DarkGrey,
        )?;
    }
    let console_status = if snapshot.connection_error.is_some() {
        "Reconnecting"
    } else if server.is_none() {
        "No server"
    } else if snapshot.active_server_running {
        "Running"
    } else {
        "Stopped"
    };
    if console_expanded {
        let top = height.saturating_sub(11);
        paint(
            out,
            main_x,
            top,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
        paint(
            out,
            main_x,
            top + 1,
            main_w,
            &format!("Console  {console_status}  [c Collapse]"),
            Color::DarkGrey,
        )?;
        let lines = snapshot.console_lines.as_deref().unwrap_or(&[]);
        let visible = height.saturating_sub(top + 4).min(6) as usize;
        for (index, line) in lines.iter().rev().take(visible).rev().enumerate() {
            paint(
                out,
                main_x,
                top + 2 + index as u16,
                main_w,
                &line.text,
                Color::White,
            )?;
        }
        paint(
            out,
            main_x,
            height - 2,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
    } else {
        paint(
            out,
            main_x,
            height - 6,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
        paint(
            out,
            main_x,
            height - 5,
            main_w,
            &format!("Console  {console_status}  [c Expand]"),
            Color::DarkGrey,
        )?;
        paint(
            out,
            main_x,
            height - 4,
            main_w,
            &"─".repeat(main_w as usize),
            Color::DarkGrey,
        )?;
    }
    paint(
        out,
        main_x,
        height - 3,
        main_w,
        if message.is_empty() {
            snapshot.connection_error.as_deref().unwrap_or("")
        } else {
            message
        },
        Color::DarkGrey,
    )?;
    paint(
        out,
        main_x,
        height - 2,
        main_w,
        "1–7 Switch tab   s Start/Stop   c Console   a Agents   m Manage   Esc Back",
        Color::DarkGrey,
    )?;
    Ok(())
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

fn paint_microsoft_link(out: &mut Stdout, x: u16, y: u16, width: u16, url: &str) -> io::Result<()> {
    const MICROSOFT_LINK: &str = "https://www.microsoft.com/link";
    if url != MICROSOFT_LINK {
        return paint(out, x, y, width, url, Color::White);
    }
    let visible: String = url.chars().take(width as usize).collect();
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Underlined),
        Print("\x1b]8;;https://www.microsoft.com/link\x1b\\"),
        Print(visible),
        Print("\x1b]8;;\x1b\\"),
        SetAttribute(Attribute::NoUnderline),
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
