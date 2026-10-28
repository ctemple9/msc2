//! Server identity, runtime, and service summary for the selected host server.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    GeyserConfigResponseDto, JavaConfigResponseDto, RamConfigResponseDto,
    ServerDirectorySizeResponseDto, ServerDto, ServerEulaResultDto, SettingsResponseDto,
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
}

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
    pub(super) fn needs_refresh(&self) -> bool {
        self.refresh_needed
    }
    pub(super) fn refreshed(&mut self) {
        self.refresh_needed = false;
    }
    pub(super) async fn handle(&mut self, key: KeyCode, message: &mut String) -> bool {
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
                self.selected = (self.selected + 1).min(self.fields().len().saturating_sub(1))
            }
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(6),
            KeyCode::PageDown => {
                self.selected = (self.selected + 6).min(self.fields().len().saturating_sub(1))
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(field) = self.fields().get(self.selected).copied() {
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
            paint(
                out,
                x + 2,
                y + 7,
                w - 4,
                &format!(
                    "Xbox Broadcast: {}",
                    if self.server.xbox_broadcast_enabled == Some(true) {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ),
                Color::White,
            )?;
            paint(
                out,
                x + 2,
                y + 9,
                w - 4,
                &format!(
                    "Playit: {}",
                    if self.server.playit_enabled == Some(true) {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ),
                Color::White,
            )?;
            paint(
                out,
                x + 2,
                y + 12,
                w - 4,
                "Setup and start controls are coming in the Services step.",
                Color::DarkGrey,
            )?;
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
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "Tab/1-3 Switch  ↑↓ Select  Enter Edit  Esc Close",
            Color::DarkGrey,
        )?;
        Ok(())
    }
}
