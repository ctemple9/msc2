//! The first start is driven by the agent's two-pass operation. The TUI only
//! advances after the operation and the stopped runtime agree on each pass.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    BroadcastStatusDto, ConnectivityResponseDto, OperationDto, OperationStateDto, PlayitStatusDto,
    RemoteApiStatus, ServerDto, SimpleResultDto,
};
use msc_infrastructure::console_buffer::ConsoleLine;
use serde_json::{Value, json};

use super::{box_outline, paint};
use crate::cli::transport::SharedClient;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Prepare,
    PassOne,
    WaitFirstStop,
    Connections,
    PassTwo,
    WaitFinalStop,
    Complete,
    Failed,
}

pub(super) struct InitiateUi {
    server: ServerDto,
    summary_only: bool,
    phase: Phase,
    eula_accepted: bool,
    operation_id: Option<String>,
    status: String,
    error: String,
    playit: Option<PlayitStatusDto>,
    broadcast: Option<BroadcastStatusDto>,
    connectivity: Option<ConnectivityResponseDto>,
    connection_selected: usize,
    service_request: Option<bool>,
    console: Vec<ConsoleLine>,
    safety_cap: bool,
    ordinary_start: bool,
    hide_requested: bool,
    pass_two_stop_requested: bool,
}

impl InitiateUi {
    pub(super) fn new(server: ServerDto) -> Self {
        Self {
            server,
            summary_only: false,
            phase: Phase::Prepare,
            eula_accepted: false,
            operation_id: None,
            status: String::new(),
            error: String::new(),
            playit: None,
            broadcast: None,
            connectivity: None,
            connection_selected: 0,
            service_request: None,
            console: Vec::new(),
            safety_cap: false,
            ordinary_start: false,
            hide_requested: false,
            pass_two_stop_requested: false,
        }
    }

    pub(super) async fn connection_summary(server: ServerDto) -> Self {
        let mut summary = Self::new(server);
        summary.summary_only = true;
        summary.phase = Phase::Complete;
        summary.refresh_connections().await;
        summary
    }
    pub(super) fn server_id(&self) -> &str {
        &self.server.id
    }

    pub(super) fn server(&self) -> &ServerDto {
        &self.server
    }

    pub(super) fn take_hide_request(&mut self) -> bool {
        std::mem::take(&mut self.hide_requested)
    }

    pub(super) fn take_service_request(&mut self) -> Option<bool> {
        self.service_request.take()
    }

    async fn refresh_connections(&mut self) {
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        if self.server.playit_enabled == Some(true) {
            self.playit = client.get_json("/v1/playit").await.ok();
        }
        if self.server.xbox_broadcast_enabled == Some(true) {
            self.broadcast = client.get_json("/v1/broadcast/status").await.ok();
        }
        self.connectivity = client.get_json("/v1/connectivity").await.ok();
    }

    async fn refresh_server(&mut self) {
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        if let Ok(servers) = client.get_json::<Vec<ServerDto>>("/v1/servers").await
            && let Some(server) = servers
                .into_iter()
                .find(|server| server.id == self.server.id)
        {
            self.server = server;
        }
    }

    async fn runtime_stopped(&self) -> Result<bool, String> {
        let client = SharedClient::connect_local()
            .await
            .map_err(|e| e.to_string())?;
        let state: RemoteApiStatus = client
            .get_json("/v1/status")
            .await
            .map_err(|e| e.to_string())?;
        Ok(!state.running)
    }

    async fn start_pass(&mut self, phase: Phase) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            if phase == Phase::PassTwo && self.server.playit_enabled == Some(true) {
                let _: Value = client.post_json("/v1/playit/start", &json!({})).await?;
            }
            client
                .post_json::<_, SimpleResultDto>("/v1/start", &json!({}))
                .await
        }
        .await;
        match result {
            Ok(accepted) => {
                if let Some(id) = accepted.operation_id {
                    self.operation_id = Some(id);
                    self.phase = phase;
                    self.status = if phase == Phase::PassOne {
                        "Creating the server files…".into()
                    } else {
                        "Checking the server and its connections…".into()
                    };
                } else {
                    self.fail("The agent did not return a start operation.".into());
                }
            }
            Err(error) => self.fail(error.to_string()),
        }
    }

    async fn begin(&mut self) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            if self.server.server_type == "java" {
                let _: Value = client
                    .post_json("/v1/servers/eula", &json!({"serverId": self.server.id}))
                    .await?;
            }
            Ok::<(), crate::cli::CliError>(())
        }
        .await;
        match result {
            Ok(()) => self.start_pass(Phase::PassOne).await,
            Err(error) => self.fail(error.to_string()),
        }
    }

    fn fail(&mut self, error: String) {
        self.error = error;
        self.phase = Phase::Failed;
    }

    fn operation_flag(operation: &OperationDto, name: &str) -> bool {
        operation
            .result
            .as_ref()
            .and_then(|value| value.get(name))
            .is_some_and(|value| value.as_bool() == Some(true) || value.as_str() == Some("true"))
    }

    async fn stop_if_running(&mut self) {
        match self.runtime_stopped().await {
            Ok(true) => self.fail("The server already has a generated world. Its ordinary start has stopped; configure connections in Services.".into()),
            Ok(false) => {
                let result = async {
                    let client = SharedClient::connect_local().await?;
                    client
                        .post_json::<_, SimpleResultDto>("/v1/stop", &json!({}))
                        .await
                }
                .await;
                if let Err(error) = result {
                    self.fail(error.to_string());
                } else {
                    self.phase = Phase::WaitFirstStop;
                    self.status = "Stopping the first run before connection setup…".into();
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub(super) async fn poll(&mut self) {
        if self.phase == Phase::Connections {
            self.refresh_server().await;
            self.refresh_connections().await;
            if self.playit_ready() && self.broadcast_ready() {
                self.start_pass(Phase::PassTwo).await;
            }
            return;
        }
        if matches!(self.phase, Phase::WaitFirstStop | Phase::WaitFinalStop) {
            if let Ok(true) = self.runtime_stopped().await {
                if self.phase == Phase::WaitFirstStop {
                    if self.ordinary_start {
                        self.fail("The server already has a generated world. Its ordinary start has stopped; configure connections in Services.".into());
                    } else {
                        self.phase = Phase::Connections;
                        self.refresh_connections().await;
                    }
                } else {
                    self.phase = Phase::Complete;
                    self.refresh_connections().await;
                }
            }
            return;
        }
        if !matches!(self.phase, Phase::PassOne | Phase::PassTwo) {
            return;
        }
        let Some(id) = self.operation_id.clone() else {
            return;
        };
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        if let Ok(lines) = client
            .get_json::<Vec<ConsoleLine>>("/v1/console/tail?n=20")
            .await
        {
            self.console = lines;
        }
        let Ok(operation) = client
            .get_json::<OperationDto>(&format!("/v1/operations/{id}"))
            .await
        else {
            return;
        };
        if let Some(line) = &operation.status_line {
            self.status = line.clone();
        }
        if self.phase == Phase::PassTwo
            && !self.pass_two_stop_requested
            && operation
                .status_line
                .as_deref()
                .is_some_and(|line| line.to_ascii_lowercase().contains("ready"))
        {
            self.refresh_connections().await;
            let playit_ready = self.server.playit_enabled != Some(true)
                || self.playit.as_ref().is_some_and(|status| {
                    status.is_running
                        && if self.server.server_type == "bedrock" {
                            status.bedrock_address.is_some()
                        } else {
                            status.java_address.is_some()
                        }
                });
            let broadcast_ready = self.server.xbox_broadcast_enabled != Some(true)
                || self.broadcast.as_ref().is_some_and(|status| {
                    status.authenticated
                        && (status.xbox_broadcast_running || status.bedrock_broadcast_running)
                });
            if playit_ready && broadcast_ready {
                match client
                    .post_json::<_, SimpleResultDto>("/v1/stop", &json!({}))
                    .await
                {
                    Ok(_) => {
                        self.pass_two_stop_requested = true;
                        self.phase = Phase::WaitFinalStop;
                        self.status = "Connections are ready. Stopping the server…".into();
                        return;
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
        }
        match operation.state {
            OperationStateDto::Failed | OperationStateDto::Cancelled => {
                self.fail(operation.error.map_or_else(
                    || "The server stopped before initiation finished.".into(),
                    |error| error.message,
                ));
            }
            OperationStateDto::Succeeded if self.phase == Phase::PassOne => {
                if operation
                    .result
                    .as_ref()
                    .and_then(|value| value.get("firstStartComplete"))
                    .is_some()
                    && !Self::operation_flag(&operation, "firstStartComplete")
                {
                    self.fail(
                        "The server stopped, but first-start completion could not be saved.".into(),
                    );
                } else if Self::operation_flag(&operation, "firstStartComplete") {
                    self.phase = Phase::WaitFinalStop;
                    self.status = "Confirming the server has stopped…".into();
                } else if Self::operation_flag(&operation, "firstStartPass1Complete") {
                    self.phase = Phase::WaitFirstStop;
                    self.status = "Pass one complete. Confirming the server has stopped…".into();
                } else {
                    // A generated world from an interrupted attempt can make
                    // Start behave as an ordinary launch. Never run a second
                    // ordinary start while the first-start coordinator is absent.
                    self.ordinary_start = true;
                    self.stop_if_running().await;
                }
            }
            OperationStateDto::Succeeded if self.phase == Phase::PassTwo => {
                if !Self::operation_flag(&operation, "firstStartComplete") {
                    self.fail(
                        "The server stopped, but first-start completion could not be saved.".into(),
                    );
                } else {
                    self.safety_cap = Self::operation_flag(&operation, "firstStartSafetyCap");
                    self.phase = Phase::WaitFinalStop;
                    self.status = "Checking that the second run has stopped…".into();
                }
            }
            _ => {}
        }
    }

    fn playit_ready(&self) -> bool {
        self.server.playit_enabled != Some(true)
            || self
                .playit
                .as_ref()
                .is_some_and(|status| status.has_secret_key)
    }

    fn broadcast_ready(&self) -> bool {
        self.server.xbox_broadcast_enabled != Some(true)
            || self
                .broadcast
                .as_ref()
                .is_some_and(|status| status.has_saved_auth || status.authenticated)
    }

    async fn defer_selected(&mut self) {
        let (path, enabled) = if self.connection_selected == 0 {
            (
                "/v1/servers/playit",
                self.server.playit_enabled == Some(true),
            )
        } else {
            (
                "/v1/servers/xbox-broadcast",
                self.server.xbox_broadcast_enabled == Some(true),
            )
        };
        if !enabled {
            return;
        }
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .post_json::<_, Value>(path, &json!({"serverId": self.server.id, "enabled": false}))
                .await
        }
        .await;
        match result {
            Ok(_) => {
                self.status = if self.connection_selected == 0 {
                    "Playit deferred. Enable it later in Services."
                } else {
                    "Xbox Broadcast deferred. Enable it later in Services."
                }
                .into();
                self.refresh_server().await;
            }
            Err(error) => self.error = error.to_string(),
        }
    }

    fn open_service(&mut self, broadcast: bool) {
        if broadcast && self.broadcast_ready() {
            self.error.clear();
            self.status = "Xbox Broadcast is already configured on this host.".into();
        } else if broadcast && !self.playit_ready() {
            self.error = "Finish or defer Playit before setting up Xbox Broadcast.".into();
        } else {
            self.error.clear();
            self.service_request = Some(broadcast);
        }
    }

    pub(super) async fn handle(&mut self, key: KeyCode) -> bool {
        if key == KeyCode::Esc
            && matches!(
                self.phase,
                Phase::Prepare
                    | Phase::PassOne
                    | Phase::WaitFirstStop
                    | Phase::Connections
                    | Phase::PassTwo
                    | Phase::WaitFinalStop
                    | Phase::Failed
            )
        {
            self.hide_requested = true;
            return false;
        }
        match self.phase {
            Phase::Prepare => match key {
                KeyCode::Char(' ') if self.server.server_type == "java" => {
                    self.eula_accepted = !self.eula_accepted
                }
                KeyCode::Enter if self.server.server_type != "java" || self.eula_accepted => {
                    self.begin().await
                }
                _ => {}
            },
            Phase::Connections => match key {
                KeyCode::Up | KeyCode::Down => {
                    self.connection_selected = 1 - self.connection_selected
                }
                KeyCode::Char('p') => {
                    self.connection_selected = 0;
                    self.open_service(false);
                }
                KeyCode::Char('b') => {
                    self.connection_selected = 1;
                    self.open_service(true);
                }
                KeyCode::Enter => self.open_service(self.connection_selected == 1),
                KeyCode::Char('d') => self.defer_selected().await,
                KeyCode::Char('r') => {
                    self.refresh_server().await;
                    self.refresh_connections().await;
                }
                KeyCode::Char('c') if self.playit_ready() && self.broadcast_ready() => {
                    self.start_pass(Phase::PassTwo).await;
                }
                KeyCode::Char('c') => {
                    self.error = "Set up or defer each enabled connection before continuing.".into()
                }
                _ => {}
            },
            Phase::Complete => {
                if matches!(key, KeyCode::Esc | KeyCode::Enter) {
                    return true;
                }
            }
            Phase::Failed => {}
            _ => {}
        }
        false
    }

    pub(super) fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        let w = width.saturating_sub(36).clamp(54, 102);
        let h = height.saturating_sub(12).clamp(20, 32);
        let x = width.saturating_sub(w) / 2;
        let y = height.saturating_sub(h) / 2;
        for line in y..y + h {
            paint(out, x, line, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(
            out,
            x + 2,
            y + 1,
            w - 4,
            &format!(
                "{} — {}",
                if self.summary_only {
                    "HOW TO CONNECT"
                } else {
                    "INITIATE"
                },
                self.server.name
            ),
            Color::White,
        )?;
        match self.phase {
            Phase::Prepare => {
                paint(out, x + 2, y + 3, w - 4, "ONE-TIME SETUP", Color::DarkGrey)?;
                paint(
                    out,
                    x + 2,
                    y + 5,
                    w - 4,
                    "MSC starts this server to create its files, then stops it.",
                    Color::White,
                )?;
                paint(
                    out,
                    x + 2,
                    y + 7,
                    w - 4,
                    "If connections are enabled, a second run checks them and stops again.",
                    Color::White,
                )?;
                if self.server.server_type == "java" {
                    paint(
                        out,
                        x + 2,
                        y + 10,
                        w - 4,
                        &format!(
                            "{} I accept the Minecraft server EULA",
                            if self.eula_accepted { "[x]" } else { "[ ]" }
                        ),
                        Color::Green,
                    )?;
                    paint(
                        out,
                        x + 2,
                        y + 12,
                        w - 4,
                        "Space Toggle  Enter Continue  Esc Later",
                        Color::DarkGrey,
                    )?;
                } else {
                    paint(
                        out,
                        x + 2,
                        y + 12,
                        w - 4,
                        "Enter Continue  Esc Later",
                        Color::DarkGrey,
                    )?;
                }
            }
            Phase::PassOne | Phase::WaitFirstStop | Phase::PassTwo | Phase::WaitFinalStop => {
                let pass = if matches!(self.phase, Phase::PassOne | Phase::WaitFirstStop) {
                    "PASS 1 OF 2 · SERVER FILES"
                } else {
                    "PASS 2 OF 2 · CONNECTIONS"
                };
                paint(out, x + 2, y + 3, w - 4, pass, Color::DarkGrey)?;
                paint(out, x + 2, y + 5, w - 4, &self.status, Color::Green)?;
                paint(
                    out,
                    x + 2,
                    y + 8,
                    w - 4,
                    "The agent owns readiness and stops the server at the end of this pass.",
                    Color::DarkGrey,
                )?;
                let console_y = y + 11;
                paint(out, x + 2, console_y, w - 4, "CONSOLE", Color::DarkGrey)?;
                for (index, line) in self
                    .console
                    .iter()
                    .rev()
                    .take((h - 16) as usize)
                    .rev()
                    .enumerate()
                {
                    paint(
                        out,
                        x + 2,
                        console_y + 2 + index as u16,
                        w - 4,
                        &line.text,
                        Color::White,
                    )?;
                }
            }
            Phase::Connections => {
                paint(
                    out,
                    x + 2,
                    y + 3,
                    w - 4,
                    "PASS 1 COMPLETE · CHOOSE CONNECTIONS",
                    Color::DarkGrey,
                )?;
                paint(
                    out,
                    x + 2,
                    y + 5,
                    w - 4,
                    "Set up enabled services before the second run. Sign-in time is outside that run.",
                    Color::White,
                )?;
                let playit = if self.server.playit_enabled != Some(true) {
                    "Deferred"
                } else if self.playit_ready() {
                    "Configured"
                } else {
                    "Setup required"
                };
                let broadcast = if self.server.xbox_broadcast_enabled != Some(true) {
                    "Deferred"
                } else if self.broadcast_ready() {
                    "Configured on this host"
                } else {
                    "Setup required"
                };
                for (index, (name, state)) in [("Playit", playit), ("Xbox Broadcast", broadcast)]
                    .iter()
                    .enumerate()
                {
                    paint(
                        out,
                        x + 2,
                        y + 9 + index as u16 * 3,
                        w - 4,
                        &format!(
                            "{} {name:<18} {state}",
                            if self.connection_selected == index {
                                ">"
                            } else {
                                " "
                            }
                        ),
                        if self.connection_selected == index {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                }
                paint(
                    out,
                    x + 2,
                    y + 17,
                    w - 4,
                    "Enter Set up/Manage  d Defer selected  c Continue  Esc Hide",
                    Color::DarkGrey,
                )?;
                paint(out, x + 2, y + 15, w - 4, &self.status, Color::DarkGrey)?;
                if !self.error.is_empty() {
                    paint(out, x + 2, y + 18, w - 4, &self.error, Color::Yellow)?;
                }
            }
            Phase::Complete => {
                paint(
                    out,
                    x + 2,
                    y + 3,
                    w - 4,
                    if self.summary_only {
                        "CONNECTION INFO"
                    } else if self.safety_cap {
                        "FIRST START STOPPED AT SAFETY LIMIT"
                    } else {
                        "FIRST START COMPLETE"
                    },
                    if self.safety_cap {
                        Color::Yellow
                    } else {
                        Color::Green
                    },
                )?;
                paint(
                    out,
                    x + 2,
                    y + 5,
                    w - 4,
                    if self.summary_only {
                        "Use the applicable address below while this server is running."
                    } else if self.safety_cap {
                        "The server stopped after 10 minutes. Check unresolved connections."
                    } else {
                        "Server files and connection choices are saved. The server is stopped."
                    },
                    Color::White,
                )?;
                let port = if self.server.server_type == "bedrock" {
                    self.server.bedrock_port
                } else {
                    self.server.game_port
                };
                let mut connections = Vec::new();
                if let Some(port) = port {
                    connections.push((
                        "Local (same Wi-Fi)",
                        format!(
                            "{}:{port}",
                            self.server.host_address.as_deref().unwrap_or("localhost")
                        ),
                    ));
                }
                if let Some(playit) = &self.playit {
                    let address = if self.server.server_type == "bedrock" {
                        &playit.bedrock_address
                    } else {
                        &playit.java_address
                    };
                    if let Some(address) = address {
                        connections.push(("Playit (any network)", address.clone()));
                    }
                }
                if let Some(connectivity) = &self.connectivity
                    && let Some(address) = connectivity.join_address.as_deref()
                {
                    let method = if connectivity.join_address_source == "duckdns" {
                        "DuckDNS"
                    } else {
                        "port forwarding"
                    };
                    connections.push(("Public (different Wi-Fi)", format!("{address} · {method}")));
                }
                if self.server.xbox_broadcast_enabled == Some(true) {
                    let gamertag = self
                        .broadcast
                        .as_ref()
                        .and_then(|status| status.gamertag.as_deref())
                        .unwrap_or("Signed in");
                    connections.push(("Xbox Broadcast", gamertag.to_string()));
                }
                for (index, (label, value)) in connections.iter().enumerate() {
                    paint(
                        out,
                        x + 2,
                        y + 8 + index as u16 * 2,
                        w - 4,
                        &format!("{label:<26}{value}"),
                        Color::White,
                    )?;
                }
                paint(
                    out,
                    x + 2,
                    y + 17,
                    w - 4,
                    if self.summary_only {
                        "Console players: add the Xbox Broadcast gamertag as a friend. Enter/Esc Close"
                    } else {
                        "Future starts are manual. Enter/Esc Done"
                    },
                    Color::DarkGrey,
                )?;
            }
            Phase::Failed => {
                paint(
                    out,
                    x + 2,
                    y + 3,
                    w - 4,
                    "INITIATION STOPPED",
                    Color::Yellow,
                )?;
                paint(out, x + 2, y + 5, w - 4, &self.error, Color::White)?;
                paint(
                    out,
                    x + 2,
                    y + 9,
                    w - 4,
                    "Check the console and server status before trying again.",
                    Color::DarkGrey,
                )?;
                paint(out, x + 2, y + 12, w - 4, "Esc Close", Color::DarkGrey)?;
            }
        }
        Ok(())
    }
}
