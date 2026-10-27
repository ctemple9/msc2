//! Component inventory and actions for the selected server.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    AddonRemoveResultDto, AddonUpdateResultDto, BroadcastJarDownloadResultDto,
    ComponentUpdateResultDto, OperationDto, OperationStateDto, ServerXboxBroadcastResultDto,
    VersionChangeResultDto, VersionsResponseDto,
};
use serde_json::json;

use super::{Snapshot, paint};
use crate::cli::transport::SharedClient;

#[derive(Default)]
pub(super) struct ComponentsUi {
    selected: usize,
    action: usize,
    mode: Mode,
    versions: Option<VersionsResponseDto>,
    refresh_needed: bool,
    pending_operation: Option<String>,
}

#[derive(Default)]
enum Mode {
    #[default]
    List,
    Actions,
    Versions(usize),
    ConfirmRemove,
}

#[derive(Clone)]
enum Row {
    Software,
    Addon(usize),
    Broadcast,
}

fn rows(snapshot: &Snapshot) -> Vec<Row> {
    if snapshot.active_server_id.is_none() {
        return Vec::new();
    }
    let mut rows = vec![Row::Software];
    if let Some(addons) = &snapshot.addons {
        rows.extend((0..addons.addons.len()).map(Row::Addon));
    }
    // Broadcast is configurable for Java and Bedrock servers alike.
    rows.push(Row::Broadcast);
    rows
}

impl ComponentsUi {
    pub(super) fn needs_refresh(&self) -> bool {
        self.refresh_needed
    }
    pub(super) fn refreshed(&mut self) {
        self.refresh_needed = false;
    }

    pub(super) async fn poll_operation(&mut self, message: &mut String) {
        let Some(id) = self.pending_operation.as_deref() else {
            return;
        };
        let Ok(client) = SharedClient::connect_local().await else {
            return;
        };
        let Ok(operation) = client
            .get_json::<OperationDto>(&format!("/v1/operations/{id}"))
            .await
        else {
            return;
        };
        match operation.state {
            OperationStateDto::Succeeded => {
                *message = "Component operation finished.".into();
                self.pending_operation = None;
                self.refresh_needed = true;
            }
            OperationStateDto::Failed | OperationStateDto::Cancelled => {
                *message = operation.error.map_or_else(
                    || "Component operation failed.".into(),
                    |error| error.message,
                );
                self.pending_operation = None;
                self.refresh_needed = true;
            }
            _ => {
                *message = operation
                    .status_line
                    .unwrap_or_else(|| "Component operation running…".into())
            }
        }
    }

    pub(super) async fn handle(
        &mut self,
        key: KeyCode,
        snapshot: &Snapshot,
        message: &mut String,
    ) -> bool {
        let all_rows = rows(snapshot);
        self.selected = self.selected.min(all_rows.len().saturating_sub(1));
        if let Mode::Versions(index) = &mut self.mode {
            match key {
                KeyCode::Esc => self.mode = Mode::List,
                KeyCode::Up | KeyCode::Left => *index = index.saturating_sub(1),
                KeyCode::Down | KeyCode::Right => {
                    *index = (*index + 1).min(
                        self.versions
                            .as_ref()
                            .map_or(0, |versions| versions.versions.len().saturating_sub(1)),
                    )
                }
                KeyCode::Enter => {
                    if snapshot.active_server_running {
                        *message = "Stop the server before changing its version.".into();
                    } else if let Some(version) = self
                        .versions
                        .as_ref()
                        .and_then(|versions| versions.versions.get(*index))
                    {
                        let id = version.id.clone();
                        let loader = version.loader_version.clone();
                        self.mode = Mode::List;
                        match SharedClient::connect_local().await {
                            Ok(client) => match client
                                .post_json::<_, VersionChangeResultDto>(
                                    "/v1/components/version",
                                    &json!({"versionId":id,"loaderVersion":loader}),
                                )
                                .await
                            {
                                Ok(result) => {
                                    *message = result.message;
                                    self.pending_operation = result.operation_id;
                                    self.refresh_needed = true;
                                }
                                Err(error) => *message = error.to_string(),
                            },
                            Err(error) => *message = error.to_string(),
                        }
                    }
                }
                _ => {}
            }
            return true;
        }
        if matches!(self.mode, Mode::ConfirmRemove) {
            self.mode = Mode::List;
            if matches!(
                key,
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter
            ) && let Some(Row::Addon(index)) = all_rows.get(self.selected)
                && let Some(addon) = snapshot
                    .addons
                    .as_ref()
                    .and_then(|addons| addons.addons.get(*index))
            {
                match SharedClient::connect_local().await {
                    Ok(client) => match client
                        .post_json::<_, AddonRemoveResultDto>(
                            "/v1/components/remove",
                            &json!({"jarStem":addon.jar_stem}),
                        )
                        .await
                    {
                        Ok(result) => {
                            *message = result.message;
                            self.refresh_needed = true;
                        }
                        Err(error) => *message = error.to_string(),
                    },
                    Err(error) => *message = error.to_string(),
                }
            }
            return true;
        }
        if matches!(self.mode, Mode::Actions) {
            let row = all_rows.get(self.selected);
            let count = match row {
                Some(Row::Addon(_)) => 3,
                Some(Row::Broadcast) => 2,
                _ => 1,
            };
            match key {
                KeyCode::Esc => self.mode = Mode::List,
                KeyCode::Up => self.action = self.action.saturating_sub(1),
                KeyCode::Down => self.action = (self.action + 1).min(count - 1),
                KeyCode::Enter => match row {
                    Some(Row::Software) => {
                        self.mode = Mode::List;
                        match SharedClient::connect_local().await {
                            Ok(client) => {
                                match client.get_json::<VersionsResponseDto>("/v1/versions").await {
                                    Ok(versions)
                                        if versions.supports_versions
                                            && !versions.versions.is_empty() =>
                                    {
                                        let current = versions.current_version.as_deref();
                                        let index = versions
                                            .versions
                                            .iter()
                                            .position(|entry| {
                                                Some(entry.mc_version.as_str()) == current
                                            })
                                            .unwrap_or(0);
                                        self.versions = Some(versions);
                                        self.mode = Mode::Versions(index);
                                    }
                                    Ok(versions) => {
                                        *message = versions.note.unwrap_or_else(|| {
                                            "Version changes are unavailable for this server."
                                                .into()
                                        })
                                    }
                                    Err(error) => *message = error.to_string(),
                                }
                            }
                            Err(error) => *message = error.to_string(),
                        }
                    }
                    Some(Row::Addon(index)) => {
                        if let Some(addon) = snapshot
                            .addons
                            .as_ref()
                            .and_then(|addons| addons.addons.get(*index))
                        {
                            match self.action {
                                0 | 1 => {
                                    let managed = if addon
                                        .jar_stem
                                        .to_ascii_lowercase()
                                        .starts_with("geyser")
                                    {
                                        Some("geyser")
                                    } else if addon
                                        .jar_stem
                                        .to_ascii_lowercase()
                                        .starts_with("floodgate")
                                    {
                                        Some("floodgate")
                                    } else {
                                        None
                                    };
                                    let body = if self.action == 0 {
                                        json!({"jarStem":addon.jar_stem,"enabled":!addon.is_enabled})
                                    } else if let Some(component) = managed {
                                        json!({"component":component})
                                    } else {
                                        json!({"jarStem":addon.jar_stem})
                                    };
                                    self.mode = Mode::List;
                                    match SharedClient::connect_local().await {
                                        Ok(client) if self.action == 1 && managed.is_some() => {
                                            match client
                                                .post_json::<_, ComponentUpdateResultDto>(
                                                    "/v1/components/update",
                                                    &body,
                                                )
                                                .await
                                            {
                                                Ok(result) => {
                                                    *message = result.message;
                                                    self.refresh_needed = true;
                                                }
                                                Err(error) => *message = error.to_string(),
                                            }
                                        }
                                        Ok(client) => match client
                                            .post_json::<_, AddonUpdateResultDto>(
                                                "/v1/components/update",
                                                &body,
                                            )
                                            .await
                                        {
                                            Ok(result) => {
                                                *message = result.result;
                                                self.pending_operation = result.operation_id;
                                                self.refresh_needed = true;
                                            }
                                            Err(error) => *message = error.to_string(),
                                        },
                                        Err(error) => *message = error.to_string(),
                                    }
                                }
                                _ => self.mode = Mode::ConfirmRemove,
                            }
                        }
                    }
                    Some(Row::Broadcast) => {
                        self.mode = Mode::List;
                        if self.action == 0 {
                            if let Some(id) = &snapshot.active_server_id
                                && let Some(server) =
                                    snapshot.servers.as_ref().and_then(|servers| {
                                        servers.iter().find(|server| &server.id == id)
                                    })
                            {
                                let enabled = !server.xbox_broadcast_enabled.unwrap_or(false);
                                match SharedClient::connect_local().await {
                                    Ok(client) => match client
                                        .post_json::<_, ServerXboxBroadcastResultDto>(
                                            "/v1/servers/xbox-broadcast",
                                            &json!({"serverId":id,"enabled":enabled}),
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            *message = result.message;
                                            self.refresh_needed = true;
                                        }
                                        Err(error) => *message = error.to_string(),
                                    },
                                    Err(error) => *message = error.to_string(),
                                }
                            }
                        } else {
                            match SharedClient::connect_local().await {
                                Ok(client) => match client
                                    .post_json::<_, BroadcastJarDownloadResultDto>(
                                        "/v1/broadcast/download-jar",
                                        &json!({}),
                                    )
                                    .await
                                {
                                    Ok(result) => {
                                        *message = result.message;
                                        self.pending_operation = result.operation_id;
                                        self.refresh_needed = true;
                                    }
                                    Err(error) => *message = error.to_string(),
                                },
                                Err(error) => *message = error.to_string(),
                            }
                        }
                    }
                    None => self.mode = Mode::List,
                },
                _ => {}
            }
            return true;
        }
        match key {
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(all_rows.len().saturating_sub(1));
                true
            }
            KeyCode::Enter => {
                if !all_rows.is_empty() {
                    self.mode = Mode::Actions;
                    self.action = 0;
                }
                true
            }
            _ => false,
        }
    }

    pub(super) fn draw(
        &self,
        out: &mut Stdout,
        snapshot: &Snapshot,
        x: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        if snapshot.active_server_id.is_none() {
            return paint(
                out,
                x,
                7,
                width,
                "Select or create a server to view components.",
                Color::DarkGrey,
            );
        }
        let all_rows = rows(snapshot);
        paint(out, x, 6, width, "COMPONENTS", Color::DarkGrey)?;
        paint(out, x, 8, width, "SERVER", Color::DarkGrey)?;
        let server_name = snapshot.active_server_id.as_ref().and_then(|id| {
            snapshot
                .servers
                .as_ref()?
                .iter()
                .find(|server| &server.id == id)
        });
        let software = snapshot
            .components
            .as_ref()
            .and_then(|components| components.components.first());
        let name = software.map_or_else(
            || {
                server_name.map_or("Server software unavailable".into(), |server| {
                    server.server_type.clone()
                })
            },
            |component| component.name.clone(),
        );
        let version = software
            .and_then(|component| component.installed_label.as_deref())
            .unwrap_or("Version unavailable");
        self.row(out, x, width, 9, 0, &format!("{name} · {version}"))?;
        let mut y = 11;
        let hint_y = height.saturating_sub(8);
        if let Some(addons) = &snapshot.addons {
            if addons.server_supports_addons {
                let flavor = server_name
                    .and_then(|server| server.java_flavor.as_deref())
                    .unwrap_or("");
                let kind = if matches!(flavor, "fabric" | "forge" | "neoforge" | "quilt") {
                    "MODS"
                } else {
                    "PLUGINS"
                };
                paint(
                    out,
                    x,
                    y,
                    width,
                    &format!("{kind} · {} installed", addons.addons.len()),
                    Color::DarkGrey,
                )?;
                y += 1;
                if addons.addons.is_empty() && y < hint_y.saturating_sub(2) {
                    paint(out, x + 2, y, width - 2, "None installed.", Color::DarkGrey)?;
                    y += 1;
                }
                let visible = hint_y.saturating_sub(y + 2) as usize;
                let start = match all_rows.get(self.selected) {
                    Some(Row::Addon(index)) => (index + 1).saturating_sub(visible),
                    _ => 0,
                };
                for (index, addon) in addons.addons.iter().enumerate().skip(start).take(visible) {
                    let state = if addon.is_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    };
                    self.row(
                        out,
                        x,
                        width,
                        y,
                        index + 1,
                        &format!(
                            "{} · {} · {state}",
                            addon.display_name,
                            addon.current_version.as_deref().unwrap_or(&addon.jar_stem)
                        ),
                    )?;
                    y += 1;
                }
            }
        } else {
            paint(
                out,
                x,
                y,
                width,
                "Add-on inventory unavailable.",
                Color::DarkGrey,
            )?;
            y += 1;
        }
        if matches!(all_rows.last(), Some(Row::Broadcast)) && y + 1 < hint_y {
            paint(out, x, y, width, "CROSSPLAY", Color::DarkGrey)?;
            y += 1;
            let enabled = server_name
                .and_then(|server| server.xbox_broadcast_enabled)
                .unwrap_or(false);
            let jar = snapshot.broadcast_jar.as_ref();
            let status = match jar {
                Some(jar) if jar.installed => "JAR installed",
                Some(_) => "JAR missing",
                None => "JAR status unavailable",
            };
            self.row(
                out,
                x,
                width,
                y,
                all_rows.len() - 1,
                &format!(
                    "MCXboxBroadcast · {} · {status}",
                    if enabled { "Enabled" } else { "Disabled" }
                ),
            )?;
        }
        paint(
            out,
            x,
            hint_y,
            width,
            "↑↓ Select component   Enter Actions   Esc Back",
            Color::DarkGrey,
        )?;
        let menu_x = x + width / 2;
        match &self.mode {
            Mode::Actions => {
                paint(
                    out,
                    menu_x,
                    8,
                    width / 2,
                    "ACTIONS · Esc Close",
                    Color::DarkGrey,
                )?;
                let actions: Vec<String> = match all_rows.get(self.selected) {
                    Some(Row::Software) => vec!["Change version…".into()],
                    Some(Row::Addon(index)) => {
                        let enabled = snapshot
                            .addons
                            .as_ref()
                            .and_then(|addons| addons.addons.get(*index))
                            .is_some_and(|addon| addon.is_enabled);
                        vec![
                            if enabled { "Disable" } else { "Enable" }.into(),
                            "Update".into(),
                            "Uninstall…".into(),
                        ]
                    }
                    Some(Row::Broadcast) => {
                        let enabled = server_name
                            .and_then(|server| server.xbox_broadcast_enabled)
                            .unwrap_or(false);
                        let installed = snapshot
                            .broadcast_jar
                            .as_ref()
                            .is_some_and(|jar| jar.installed);
                        vec![
                            if enabled { "Disable" } else { "Enable" }.into(),
                            if installed {
                                "Update JAR"
                            } else {
                                "Download JAR"
                            }
                            .into(),
                        ]
                    }
                    None => Vec::new(),
                };
                for (index, action) in actions.iter().enumerate() {
                    paint(
                        out,
                        menu_x,
                        9 + index as u16,
                        width / 2,
                        &format!("{} {action}", if index == self.action { ">" } else { " " }),
                        if index == self.action {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                }
            }
            Mode::Versions(index) => {
                paint(
                    out,
                    menu_x,
                    8,
                    width / 2,
                    "VERSIONS · ↑↓ Choose · Enter Change",
                    Color::DarkGrey,
                )?;
                if let Some(versions) = &self.versions {
                    let start = index.saturating_sub(3);
                    for (offset, entry) in versions
                        .versions
                        .iter()
                        .skip(start)
                        .take(height.saturating_sub(18) as usize)
                        .enumerate()
                    {
                        let selected = start + offset == *index;
                        paint(
                            out,
                            menu_x,
                            9 + offset as u16,
                            width / 2,
                            &format!(
                                "{} {}",
                                if selected { ">" } else { " " },
                                entry.display_label
                            ),
                            if selected { Color::Green } else { Color::White },
                        )?;
                    }
                }
            }
            Mode::ConfirmRemove => paint(
                out,
                menu_x,
                9,
                width / 2,
                "Uninstall this add-on? Enter/y Yes · Esc No",
                Color::Green,
            )?,
            Mode::List => {}
        }
        Ok(())
    }

    fn row(
        &self,
        out: &mut Stdout,
        x: u16,
        width: u16,
        y: u16,
        index: usize,
        text: &str,
    ) -> io::Result<()> {
        paint(
            out,
            x + 2,
            y,
            width - 4,
            &format!("{} {text}", if index == self.selected { ">" } else { " " }),
            if index == self.selected {
                Color::Green
            } else {
                Color::White
            },
        )
    }
}
