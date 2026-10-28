//! Server identity, runtime, and service summary for the selected host server.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    BroadcastAuthPromptDto, BroadcastSimpleResultDto, BroadcastStatusDto, GeyserConfigResponseDto,
    JavaConfigResponseDto, JavaRuntimeDto, JavaRuntimeInstallResultDto, JavaRuntimesResponseDto,
    OperationDto, OperationStateDto, RamConfigResponseDto, ServerDirectorySizeResponseDto,
    ServerDto, ServerEulaResultDto, SettingsResponseDto,
};
use serde_json::{Value, json};

use super::{box_outline, paint};
use crate::cli::transport::{SharedClient, encode_query_component};

pub(super) struct ServerEditor {
    server: ServerDto,
    tab: usize,
    selected: usize,
    scroll: usize,
    edit: Option<String>,
    confirm_eula: bool,
    ram: Option<RamConfigResponseDto>,
    java: Option<JavaConfigResponseDto>,
    settings: Option<SettingsResponseDto>,
    geyser: Option<GeyserConfigResponseDto>,
    size: Option<u64>,
    eula: Option<bool>,
    refresh_needed: bool,
    java_view: JavaView,
    broadcast: Option<BroadcastStatusDto>,
    broadcast_prompt: Option<BroadcastAuthPromptDto>,
    broadcast_setup_open: bool,
    broadcast_operation_id: Option<String>,
    broadcast_enabled_before_setup: bool,
    confirm_clear_broadcast: bool,
}

#[derive(Default)]
enum JavaView {
    #[default]
    None,
    Detected {
        runtimes: Vec<JavaRuntimeDto>,
        selected: usize,
    },
    Install {
        selected: usize,
    },
    Installing {
        operation_id: String,
        major: i64,
    },
}

const INSTALL_MAJORS: [i64; 4] = [25, 21, 17, 8];

#[derive(Clone, Copy)]
enum Field {
    Name,
    Directory,
    MinRam,
    MaxRam,
    JavaPort,
    BedrockPort,
    Eula,
    JavaPath,
    JvmFlags,
}

impl ServerEditor {
    pub(super) async fn open(server: ServerDto) -> Self {
        let mut editor = Self {
            server,
            tab: 0,
            selected: 0,
            scroll: 0,
            edit: None,
            confirm_eula: false,
            ram: None,
            java: None,
            settings: None,
            geyser: None,
            size: None,
            eula: None,
            refresh_needed: false,
            java_view: JavaView::None,
            broadcast: None,
            broadcast_prompt: None,
            broadcast_setup_open: false,
            broadcast_operation_id: None,
            broadcast_enabled_before_setup: false,
            confirm_clear_broadcast: false,
        };
        editor.reload().await;
        editor
    }
    async fn reload(&mut self) {
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        let size_route = format!(
            "/v1/servers/size?serverId={}",
            encode_query_component(&self.server.id)
        );
        let eula_route = format!(
            "/v1/servers/eula?serverId={}",
            encode_query_component(&self.server.id)
        );
        let (ram, java, settings, geyser, size, eula) = tokio::join!(
            client.get_json::<RamConfigResponseDto>("/v1/config/ram"),
            client.get_json::<JavaConfigResponseDto>("/v1/config/java-runtime"),
            client.get_json::<SettingsResponseDto>("/v1/settings"),
            client.get_json::<GeyserConfigResponseDto>("/v1/config/geyser"),
            client.get_json::<ServerDirectorySizeResponseDto>(&size_route),
            client.get_json::<ServerEulaResultDto>(&eula_route),
        );
        self.ram = ram.ok();
        self.java = java.ok();
        self.settings = settings.ok();
        self.geyser = geyser.ok();
        self.size = size.ok().and_then(|size| size.size_bytes);
        self.eula = eula.ok().and_then(|eula| eula.accepted);
        self.refresh_broadcast().await;
    }
    async fn refresh_broadcast(&mut self) {
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        if let Ok(status) = client
            .get_json::<BroadcastStatusDto>("/v1/broadcast/status")
            .await
        {
            self.broadcast = Some(status);
        }
        if self.broadcast_setup_open
            && let Ok(prompt) = client
                .get_json::<BroadcastAuthPromptDto>("/v1/broadcast/auth-prompt")
                .await
        {
            self.broadcast_prompt = Some(prompt);
        }
    }

    pub(super) async fn poll_services(&mut self, message: &mut String) {
        if self.tab != 1 && !self.broadcast_setup_open {
            return;
        }
        self.refresh_broadcast().await;
        if self.broadcast_setup_open
            && let Some(id) = self.broadcast_operation_id.clone()
            && let Ok(client) = SharedClient::connect_local().await
            && let Ok(operation) = client
                .get_json::<OperationDto>(&format!("/v1/operations/{id}"))
                .await
        {
            match operation.state {
                OperationStateDto::Failed | OperationStateDto::Cancelled => {
                    self.broadcast_setup_open = false;
                    self.broadcast_operation_id = None;
                    *message = operation.error.map_or_else(
                        || "Xbox Broadcast setup failed.".into(),
                        |error| error.message,
                    );
                    if !self.broadcast_enabled_before_setup {
                        let failure = message.clone();
                        self.toggle_service(true, message).await;
                        *message = failure;
                    }
                    return;
                }
                OperationStateDto::Succeeded => self.broadcast_operation_id = None,
                _ => {}
            }
        }
        if self.broadcast_setup_open
            && self
                .broadcast
                .as_ref()
                .is_some_and(|status| status.authenticated)
        {
            self.broadcast_setup_open = false;
            self.broadcast_operation_id = None;
            self.broadcast_prompt = None;
            *message = "Xbox Broadcast signed in.".into();
        }
    }

    async fn toggle_service(&mut self, broadcast: bool, message: &mut String) {
        let enabled = if broadcast {
            !self.server.xbox_broadcast_enabled.unwrap_or(false)
        } else {
            !self.server.playit_enabled.unwrap_or(false)
        };
        let route = if broadcast {
            "/v1/servers/xbox-broadcast"
        } else {
            "/v1/servers/playit"
        };
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .post_json::<_, Value>(route, &json!({"serverId":self.server.id,"enabled":enabled}))
                .await
            {
                Ok(result) if result["success"] == true => {
                    if broadcast {
                        self.server.xbox_broadcast_enabled = Some(enabled);
                    } else {
                        self.server.playit_enabled = Some(enabled);
                    }
                    *message = result["message"].as_str().unwrap_or("Saved.").into();
                    self.refresh_needed = true;
                }
                Ok(result) => {
                    *message = result["message"]
                        .as_str()
                        .unwrap_or("Could not save.")
                        .into()
                }
                Err(error) => *message = error.to_string(),
            },
            Err(error) => *message = error.to_string(),
        }
    }

    async fn setup_broadcast(&mut self, message: &mut String) {
        let enabled_before_setup = self.server.xbox_broadcast_enabled == Some(true);
        if !enabled_before_setup {
            self.toggle_service(true, message).await;
            if self.server.xbox_broadcast_enabled != Some(true) {
                return;
            }
        }
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .post_json::<_, BroadcastSimpleResultDto>("/v1/broadcast/start", &json!({}))
                .await
            {
                Ok(result) => {
                    self.broadcast_setup_open = true;
                    self.broadcast_enabled_before_setup = enabled_before_setup;
                    self.broadcast_operation_id = result.operation_id;
                    self.broadcast_prompt = None;
                    *message = "Starting Xbox Broadcast sign-in…".into();
                    self.poll_services(message).await;
                }
                Err(error) => {
                    *message = error.to_string();
                    if !enabled_before_setup {
                        let failure = message.clone();
                        self.toggle_service(true, message).await;
                        *message = failure;
                    }
                }
            },
            Err(error) => {
                *message = error.to_string();
                if !enabled_before_setup {
                    let failure = message.clone();
                    self.toggle_service(true, message).await;
                    *message = failure;
                }
            }
        }
    }

    async fn cancel_broadcast_setup(&mut self, message: &mut String) {
        match SharedClient::connect_local().await {
            Ok(client) => {
                match client
                    .post_json::<_, BroadcastSimpleResultDto>("/v1/broadcast/stop", &json!({}))
                    .await
                {
                    Ok(_) => {
                        self.broadcast_setup_open = false;
                        self.broadcast_operation_id = None;
                        self.broadcast_prompt = None;
                        *message = "Xbox Broadcast setup stopped.".into();
                        if !self.broadcast_enabled_before_setup {
                            self.toggle_service(true, message).await;
                            if self.server.xbox_broadcast_enabled == Some(false) {
                                *message = "Xbox Broadcast setup stopped; previous disabled setting restored.".into();
                            }
                        }
                        self.refresh_broadcast().await;
                    }
                    Err(error) => *message = format!("Could not stop Xbox Broadcast: {error}"),
                }
            }
            Err(error) => *message = format!("Could not stop Xbox Broadcast: {error}"),
        }
    }

    async fn clear_broadcast(&mut self, message: &mut String) {
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .post_json::<_, BroadcastSimpleResultDto>(
                    "/v1/broadcast/credentials/clear",
                    &json!({}),
                )
                .await
            {
                Ok(_) => {
                    self.broadcast_setup_open = false;
                    self.broadcast_prompt = None;
                    *message = "Xbox Broadcast sign-in cleared for this host.".into();
                    self.refresh_broadcast().await;
                }
                Err(error) => *message = error.to_string(),
            },
            Err(error) => *message = error.to_string(),
        }
    }

    fn fields(&self) -> Vec<Field> {
        if self.tab == 2 {
            return vec![Field::JavaPath, Field::JvmFlags];
        }
        if self.tab == 1 {
            return Vec::new();
        }
        let mut fields = vec![Field::Name, Field::Directory];
        if self.server.server_type == "java" {
            fields.extend([Field::MinRam, Field::MaxRam, Field::JavaPort]);
        }
        if self.server.server_type == "bedrock"
            || self
                .geyser
                .as_ref()
                .is_some_and(|config| config.is_geyser_installed)
        {
            fields.push(Field::BedrockPort);
        }
        if self.server.server_type == "java" {
            fields.push(Field::Eula);
        }
        fields
    }
    fn port_value(&self, key: &str) -> Option<String> {
        self.settings
            .as_ref()?
            .sections
            .iter()
            .flat_map(|section| &section.fields)
            .find(|field| field.key == key)
            .map(|field| field.value.clone())
    }
    fn value(&self, field: Field) -> String {
        match field {
            Field::Name => self.server.name.clone(),
            Field::Directory => self.server.directory.clone(),
            Field::MinRam => self
                .ram
                .as_ref()
                .map_or("—".into(), |ram| format!("{} GB", ram.min_ram_gb)),
            Field::MaxRam => self
                .ram
                .as_ref()
                .map_or("—".into(), |ram| format!("{} GB", ram.max_ram_gb)),
            Field::JavaPort => self
                .port_value("server-port")
                .or_else(|| self.server.game_port.map(|port| port.to_string()))
                .unwrap_or_else(|| "—".into()),
            Field::BedrockPort => {
                if self.server.server_type == "bedrock" {
                    self.port_value("server-port")
                        .or_else(|| self.server.bedrock_port.map(|port| port.to_string()))
                        .unwrap_or_else(|| "—".into())
                } else {
                    self.geyser
                        .as_ref()
                        .and_then(|config| config.port)
                        .map_or("—".into(), |port| port.to_string())
                }
            }
            Field::Eula => {
                if self.eula == Some(true) {
                    "Accepted".into()
                } else {
                    "Not accepted".into()
                }
            }
            Field::JavaPath => self
                .java
                .as_ref()
                .and_then(|java| java.executable_path.clone())
                .unwrap_or_else(|| "java".into()),
            Field::JvmFlags => self
                .java
                .as_ref()
                .and_then(|java| java.extra_flags.clone())
                .unwrap_or_default(),
        }
    }
    fn label(field: Field) -> &'static str {
        match field {
            Field::Name => "Display Name",
            Field::Directory => "Server Directory",
            Field::MinRam => "Minimum RAM",
            Field::MaxRam => "Maximum RAM",
            Field::JavaPort => "Java Port",
            Field::BedrockPort => "Bedrock / Geyser Port",
            Field::Eula => "Minecraft EULA",
            Field::JavaPath => "Executable Path",
            Field::JvmFlags => "Extra JVM Flags",
        }
    }
    pub(super) async fn poll_operation(&mut self, message: &mut String) {
        let JavaView::Installing {
            operation_id,
            major,
        } = &self.java_view
        else {
            return;
        };
        let id = operation_id.clone();
        let major = *major;
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        let Ok(operation) = client
            .get_json::<OperationDto>(&format!("/v1/operations/{id}"))
            .await
        else {
            *message = "Could not check Java installation. Press r to retry.".into();
            return;
        };
        match operation.state {
            OperationStateDto::Succeeded => {
                *message = format!("Java {major} installed. Press d to select a runtime.");
                self.java_view = JavaView::None;
                self.reload().await;
            }
            OperationStateDto::Failed | OperationStateDto::Cancelled => {
                *message = operation
                    .error
                    .map_or_else(|| "Java installation failed.".into(), |error| error.message);
                self.java_view = JavaView::Install {
                    selected: INSTALL_MAJORS
                        .iter()
                        .position(|candidate| *candidate == major)
                        .unwrap_or(0),
                };
            }
            _ => {
                *message = operation
                    .status_line
                    .unwrap_or_else(|| format!("Installing Java {major}…"))
            }
        }
    }

    async fn detect_runtimes(&mut self, message: &mut String) {
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .get_json::<JavaRuntimesResponseDto>("/v1/java-runtimes")
                .await
            {
                Ok(result) => {
                    let selected = result
                        .runtimes
                        .iter()
                        .position(|runtime| {
                            self.java
                                .as_ref()
                                .and_then(|java| java.executable_path.as_deref())
                                == Some(runtime.executable_path.as_str())
                        })
                        .unwrap_or(0);
                    self.java_view = JavaView::Detected {
                        runtimes: result.runtimes,
                        selected,
                    };
                    message.clear();
                }
                Err(error) => *message = error.to_string(),
            },
            Err(error) => *message = error.to_string(),
        }
    }

    async fn handle_java_view(&mut self, key: KeyCode, message: &mut String) -> bool {
        match &mut self.java_view {
            JavaView::None => return false,
            JavaView::Detected { runtimes, selected } => match key {
                KeyCode::Esc => self.java_view = JavaView::None,
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(runtimes.len().saturating_sub(1)),
                KeyCode::Char('r') => self.detect_runtimes(message).await,
                KeyCode::Char('i') => self.java_view = JavaView::Install { selected: 0 },
                KeyCode::Enter => {
                    if let Some(runtime) = runtimes.get(*selected) {
                        let path = runtime.executable_path.clone();
                        self.post(
                            "/v1/config/java-runtime",
                            json!({"executablePath":path}),
                            message,
                        )
                        .await;
                        self.java_view = JavaView::None;
                    }
                }
                _ => {}
            },
            JavaView::Install { selected } => match key {
                KeyCode::Esc => self.java_view = JavaView::None,
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(INSTALL_MAJORS.len() - 1),
                KeyCode::Enter => {
                    let major = INSTALL_MAJORS[*selected];
                    match SharedClient::connect_local().await {
                        Ok(client) => match client
                            .post_json::<_, JavaRuntimeInstallResultDto>(
                                "/v1/java-runtimes/install",
                                &json!({"major":major}),
                            )
                            .await
                        {
                            Ok(result) if result.success => {
                                self.java_view = JavaView::Installing {
                                    operation_id: result.operation_id,
                                    major,
                                };
                                *message = format!("Installing Java {major}…");
                            }
                            Ok(result) => *message = result.message,
                            Err(error) => *message = error.to_string(),
                        },
                        Err(error) => *message = error.to_string(),
                    }
                }
                _ => {}
            },
            JavaView::Installing { .. } => {
                if matches!(key, KeyCode::Char('r') | KeyCode::Enter) {
                    self.poll_operation(message).await;
                }
            }
        }
        true
    }

    pub(super) fn needs_refresh(&self) -> bool {
        self.refresh_needed
    }
    pub(super) fn refreshed(&mut self) {
        self.refresh_needed = false;
    }
    pub(super) async fn handle(&mut self, key: KeyCode, message: &mut String) -> bool {
        if self.broadcast_setup_open {
            match key {
                KeyCode::Esc => self.cancel_broadcast_setup(message).await,
                KeyCode::Char('r') => self.poll_services(message).await,
                _ => {}
            }
            return false;
        }
        if self.confirm_clear_broadcast {
            self.confirm_clear_broadcast = false;
            if matches!(key, KeyCode::Char('y') | KeyCode::Char('Y')) {
                self.clear_broadcast(message).await;
            } else {
                *message = "Xbox Broadcast sign-in kept.".into();
            }
            return false;
        }
        if self.handle_java_view(key, message).await {
            return false;
        }
        if self.confirm_eula {
            self.confirm_eula = false;
            if matches!(key, KeyCode::Char('y') | KeyCode::Char('Y')) {
                self.post(
                    "/v1/servers/eula",
                    json!({"serverId":self.server.id}),
                    message,
                )
                .await;
            } else {
                *message = "EULA acceptance cancelled.".into();
            }
            return false;
        }
        if let Some(mut draft) = self.edit.take() {
            match key {
                KeyCode::Esc => *message = "Edit cancelled.".into(),
                KeyCode::Backspace => {
                    draft.pop();
                    self.edit = Some(draft);
                }
                KeyCode::Char(c) if !c.is_control() => {
                    draft.push(c);
                    self.edit = Some(draft);
                }
                KeyCode::Enter => {
                    if let Some(field) = self.fields().get(self.selected).copied() {
                        self.save(field, &draft, message).await;
                    }
                }
                _ => self.edit = Some(draft),
            }
            return false;
        }
        match key {
            KeyCode::Esc => return true,
            KeyCode::Char('d') if self.tab == 2 => self.detect_runtimes(message).await,
            KeyCode::Char('i') if self.tab == 2 => {
                self.java_view = JavaView::Install { selected: 0 }
            }
            KeyCode::Char('1'..='3') => {
                if let KeyCode::Char(digit) = key {
                    let tab = digit as usize - '1' as usize;
                    if tab < 2 || self.server.server_type == "java" {
                        self.tab = tab;
                        self.selected = 0;
                        self.scroll = 0;
                    }
                }
            }
            KeyCode::Tab => {
                self.tab = (self.tab + 1)
                    % if self.server.server_type == "java" {
                        3
                    } else {
                        2
                    };
                self.selected = 0;
                self.scroll = 0;
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                let count = if self.tab == 1 {
                    3
                } else {
                    self.fields().len()
                };
                self.selected = (self.selected + 1).min(count.saturating_sub(1))
            }
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(6),
            KeyCode::PageDown => {
                let count = if self.tab == 1 {
                    3
                } else {
                    self.fields().len()
                };
                self.selected = (self.selected + 6).min(count.saturating_sub(1))
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if self.tab == 1 {
                    match self.selected {
                        0 => self.toggle_service(true, message).await,
                        1 if self
                            .broadcast
                            .as_ref()
                            .is_some_and(|status| status.has_saved_auth) =>
                        {
                            self.confirm_clear_broadcast = true;
                            *message = "Clear Xbox Broadcast sign-in on this host for all servers? y confirms; any other key cancels.".into();
                        }
                        1 => self.setup_broadcast(message).await,
                        2 => self.toggle_service(false, message).await,
                        _ => {}
                    }
                } else if let Some(field) = self.fields().get(self.selected).copied() {
                    if matches!(field, Field::Eula) {
                        if self.eula != Some(true) {
                            self.confirm_eula = true;
                            *message =
                                "Accept the Minecraft EULA for this server? Press y to confirm."
                                    .into();
                        }
                    } else {
                        self.edit = Some(self.value(field).trim_end_matches(" GB").to_string());
                    }
                }
            }
            _ => {}
        }
        false
    }
    async fn save(&mut self, field: Field, draft: &str, message: &mut String) {
        let value = draft.trim();
        if value.is_empty() && !matches!(field, Field::JvmFlags) {
            *message = "Value cannot be empty.".into();
            return;
        }
        match field {
            Field::Name => {
                self.post(
                    "/v1/servers/rename",
                    json!({"serverId":self.server.id,"name":value}),
                    message,
                )
                .await
            }
            Field::Directory => {
                self.post(
                    "/v1/servers/directory",
                    json!({"serverId":self.server.id,"directory":value}),
                    message,
                )
                .await
            }
            Field::MinRam | Field::MaxRam => {
                let Ok(number) = value.parse::<f64>() else {
                    *message = "RAM must be a number of GB.".into();
                    return;
                };
                if !number.is_finite() || number <= 0.0 {
                    *message = "RAM must be greater than zero.".into();
                    return;
                }
                let body = if matches!(field, Field::MinRam) {
                    json!({"minRamGB":number})
                } else {
                    json!({"maxRamGB":number})
                };
                self.post("/v1/config/ram", body, message).await;
            }
            Field::JavaPort | Field::BedrockPort => {
                let Ok(port) = value.parse::<u16>() else {
                    *message = "Port must be between 1 and 65535.".into();
                    return;
                };
                if port == 0 {
                    *message = "Port must be between 1 and 65535.".into();
                    return;
                }
                if matches!(field, Field::BedrockPort) && self.server.server_type == "java" {
                    self.post("/v1/config/geyser", json!({"port":port}), message)
                        .await;
                } else {
                    self.post(
                        "/v1/settings",
                        json!({"changes":{"server-port":value}}),
                        message,
                    )
                    .await;
                }
            }
            Field::JavaPath => {
                self.post(
                    "/v1/config/java-runtime",
                    json!({"executablePath":value}),
                    message,
                )
                .await
            }
            Field::JvmFlags => {
                self.post(
                    "/v1/config/java-runtime",
                    json!({"extraFlags":value}),
                    message,
                )
                .await
            }
            Field::Eula => {}
        }
    }
    async fn post(&mut self, route: &str, body: Value, message: &mut String) {
        match SharedClient::connect_local().await {
            Ok(client) => match client.post_json::<_, Value>(route, &body).await {
                Ok(result) => {
                    if result["success"].as_bool() == Some(false) {
                        *message = result["message"]
                            .as_str()
                            .unwrap_or("Could not save.")
                            .into();
                    } else {
                        *message = result["message"].as_str().unwrap_or("Saved.").into();
                        if result["restartRequired"].as_bool() == Some(true) {
                            message.push_str(" Restart required.");
                        }
                        if let Some(name) = result["name"].as_str() {
                            self.server.name = name.into();
                        }
                        if let Some(directory) = result["directory"].as_str() {
                            self.server.directory = directory.into();
                        }
                        self.refresh_needed = true;
                        self.reload().await;
                    }
                }
                Err(error) => *message = error.to_string(),
            },
            Err(error) => *message = error.to_string(),
        }
    }
    fn draw_java_view(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        let (title, count, selected) = match &self.java_view {
            JavaView::None => return Ok(()),
            JavaView::Detected { runtimes, selected } => {
                ("DETECTED JAVA RUNTIMES", runtimes.len(), *selected)
            }
            JavaView::Install { selected } => ("INSTALL JAVA", INSTALL_MAJORS.len(), *selected),
            JavaView::Installing { major, .. } => ("INSTALLING JAVA", 1, *major as usize),
        };
        let w = width.saturating_sub(36).min(88);
        let x = (width - w) / 2;
        let h = (count as u16 * 3 + 9).min(height.saturating_sub(6)).max(12);
        let y = (height - h) / 2;
        for line in y..y + h {
            paint(out, x, line, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(out, x + 2, y + 1, w - 4, title, Color::White)?;
        match &self.java_view {
            JavaView::Detected { runtimes, .. } => {
                if runtimes.is_empty() {
                    paint(
                        out,
                        x + 2,
                        y + 4,
                        w - 4,
                        "No runtimes detected. Esc closes; i opens Install Java.",
                        Color::DarkGrey,
                    )?;
                }
                for (index, runtime) in runtimes.iter().enumerate().take(((h - 7) / 3) as usize) {
                    let row = y + 3 + index as u16 * 3;
                    let label = runtime
                        .major_version
                        .map_or("Java ?".into(), |major| format!("Java {major}"));
                    paint(
                        out,
                        x + 2,
                        row,
                        w - 4,
                        &format!(
                            "{} {}  {}",
                            if selected == index { ">" } else { " " },
                            label,
                            runtime.name
                        ),
                        if selected == index {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                    paint(
                        out,
                        x + 4,
                        row + 1,
                        w - 6,
                        &runtime.executable_path,
                        Color::DarkGrey,
                    )?;
                }
                paint(
                    out,
                    x + 2,
                    y + h - 2,
                    w - 4,
                    "↑↓ Choose  Enter Use  r Refresh  Esc Close",
                    Color::DarkGrey,
                )?;
            }
            JavaView::Install { .. } => {
                paint(
                    out,
                    x + 2,
                    y + 3,
                    w - 4,
                    "Choose the Java version for this host.",
                    Color::DarkGrey,
                )?;
                for (index, major) in INSTALL_MAJORS.iter().enumerate() {
                    paint(
                        out,
                        x + 2,
                        y + 5 + index as u16 * 2,
                        w - 4,
                        &format!("{} Java {major}", if selected == index { ">" } else { " " }),
                        if selected == index {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                }
                paint(
                    out,
                    x + 2,
                    y + h - 2,
                    w - 4,
                    "↑↓ Choose  Enter Install  Esc Cancel",
                    Color::DarkGrey,
                )?;
            }
            JavaView::Installing { major, .. } => {
                paint(
                    out,
                    x + 2,
                    y + 4,
                    w - 4,
                    &format!("Installing Java {major}. Status refreshes automatically."),
                    Color::Green,
                )?;
                paint(
                    out,
                    x + 2,
                    y + h - 2,
                    w - 4,
                    "r Check status",
                    Color::DarkGrey,
                )?;
            }
            JavaView::None => {}
        }
        Ok(())
    }

    pub(super) fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        let x = 12;
        let y = 4;
        let w = width.saturating_sub(24);
        let h = height.saturating_sub(8);
        for line in y..y + h {
            paint(out, x, line, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(
            out,
            x + 2,
            y + 1,
            w - 4,
            &format!("EDIT SERVER — {}", self.server.name),
            Color::White,
        )?;
        let tabs = if self.server.server_type == "java" {
            "1 General    2 Services    3 Java"
        } else {
            "1 General    2 Services    Bedrock"
        };
        paint(out, x + 2, y + 3, w - 4, tabs, Color::Green)?;
        let active = ["GENERAL", "SERVICES", "JAVA"][self.tab];
        paint(out, x + 2, y + 5, w - 4, active, Color::DarkGrey)?;
        if self.tab == 1 {
            let setup = self
                .broadcast
                .as_ref()
                .is_some_and(|status| status.has_saved_auth);
            let rows = [
                format!(
                    "Xbox Broadcast        {}  ·  Enter to {}",
                    if self.server.xbox_broadcast_enabled == Some(true) {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                    if self.server.xbox_broadcast_enabled == Some(true) {
                        "disable"
                    } else {
                        "enable"
                    },
                ),
                format!(
                    "Xbox Broadcast sign-in  {}  ·  Enter to {}",
                    if setup { "Saved" } else { "Not set up" },
                    if setup { "clear" } else { "set up" }
                ),
                format!(
                    "Playit                {}  ·  Enter to {}",
                    if self.server.playit_enabled == Some(true) {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                    if self.server.playit_enabled == Some(true) {
                        "disable"
                    } else {
                        "enable"
                    },
                ),
            ];
            for (index, row) in rows.iter().enumerate() {
                paint(
                    out,
                    x + 2,
                    y + 7 + index as u16 * 2,
                    w - 4,
                    &format!("{} {row}", if self.selected == index { ">" } else { " " }),
                    if self.selected == index {
                        Color::Green
                    } else {
                        Color::White
                    },
                )?;
            }
            paint(
                out,
                x + 2,
                y + 15,
                w - 4,
                "Broadcast sign-in is shared across servers on this host. Playit setup comes later.",
                Color::DarkGrey,
            )?;
            if self.confirm_clear_broadcast {
                paint(
                    out,
                    x + 2,
                    y + 17,
                    w - 4,
                    "Clear shared Xbox Broadcast sign-in? y Confirm  any other key Cancel",
                    Color::Yellow,
                )?;
            }
        } else {
            if self.tab == 2 {
                paint(
                    out,
                    x + 2,
                    y + 6,
                    w - 4,
                    "Java executable and JVM flags are shared across this host.",
                    Color::DarkGrey,
                )?;
            }
            let fields = self.fields();
            let available = h.saturating_sub(14) as usize;
            let scroll = self
                .scroll
                .max(self.selected.saturating_sub(available.saturating_sub(1)))
                .min(self.selected);
            for (index, field) in fields.iter().enumerate().skip(scroll).take(available) {
                let selected = index == self.selected;
                let value = if selected {
                    self.edit.clone().unwrap_or_else(|| self.value(*field))
                } else {
                    self.value(*field)
                };
                let row = format!(
                    "{} {:<26} {}",
                    if selected { ">" } else { " " },
                    Self::label(*field),
                    value
                );
                paint(
                    out,
                    x + 2,
                    y + 8 + (index - scroll) as u16,
                    w - 4,
                    &row,
                    if selected { Color::Green } else { Color::White },
                )?;
            }
            if self.tab == 0
                && let Some(size) = self.size
            {
                paint(
                    out,
                    x + 2,
                    h + y - 5,
                    w - 4,
                    &format!("Server folder: {:.1} MB", size as f64 / 1_000_000.0),
                    Color::DarkGrey,
                )?;
            }
        }
        if self.tab == 2 && matches!(self.java_view, JavaView::None) {
            paint(
                out,
                x + 2,
                y + h - 4,
                w - 4,
                "d Detect Java runtimes    i Install Java",
                Color::Green,
            )?;
        }
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "Tab/1-3 Switch  ↑↓ Select  Enter Edit  Esc Close",
            Color::DarkGrey,
        )?;
        self.draw_java_view(out, width, height)?;
        if self.broadcast_setup_open {
            let modal_width = width.saturating_sub(40).clamp(40, 82);
            let modal_x = (width - modal_width) / 2;
            let modal_y = (height.saturating_sub(14)) / 2;
            for line in modal_y..modal_y + 14 {
                paint(
                    out,
                    modal_x,
                    line,
                    modal_width,
                    &" ".repeat(modal_width as usize),
                    Color::Black,
                )?;
            }
            box_outline(out, modal_x, modal_y, modal_width, 14)?;
            paint(
                out,
                modal_x + 2,
                modal_y + 1,
                modal_width - 4,
                "SIGN IN TO XBOX BROADCAST",
                Color::White,
            )?;
            paint(
                out,
                modal_x + 2,
                modal_y + 3,
                modal_width - 4,
                "Open this Microsoft page in a browser and enter the device code:",
                Color::DarkGrey,
            )?;
            if let Some(prompt) = &self.broadcast_prompt {
                paint(
                    out,
                    modal_x + 2,
                    modal_y + 5,
                    modal_width - 4,
                    prompt
                        .link_url
                        .as_deref()
                        .unwrap_or("Waiting for sign-in link…"),
                    Color::White,
                )?;
                paint(
                    out,
                    modal_x + 2,
                    modal_y + 7,
                    modal_width - 4,
                    &format!(
                        "Device code: {}",
                        prompt.code.as_deref().unwrap_or("Waiting for code…")
                    ),
                    Color::Green,
                )?;
            } else {
                paint(
                    out,
                    modal_x + 2,
                    modal_y + 5,
                    modal_width - 4,
                    "Waiting for Xbox Broadcast to provide a device code…",
                    Color::DarkGrey,
                )?;
            }
            paint(
                out,
                modal_x + 2,
                modal_y + 11,
                modal_width - 4,
                "Closes after sign-in. r Refresh  Esc Stop setup",
                Color::DarkGrey,
            )?;
        }
        Ok(())
    }
}
