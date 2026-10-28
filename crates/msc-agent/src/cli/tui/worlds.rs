//! Saved world slots and their backups in the server workspace.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    BackupConfigUpdateResultDto, BackupNowResultDto, BackupRestoreResultDto, OperationDto,
    OperationStateDto, SimpleResultDto, WorldMutationResultDto,
};
use serde_json::json;

use super::{Snapshot, paint, world_settings::WorldSettingsUi};
use crate::cli::transport::SharedClient;

const WORLD_ACTIONS: [&str; 5] = [
    "Convert World…",
    "World Settings…",
    "Rename…",
    "Duplicate…",
    "Delete…",
];
const BACKUP_ACTIONS: [&str; 2] = ["Restore…", "Delete…"];

#[derive(Default)]
pub(super) struct WorldsUi {
    world_index: usize,
    backup_index: usize,
    backup_focus: bool,
    mode: Mode,
    action_index: usize,
    refresh_needed: bool,
    pending_operation: Option<String>,
    settings: Option<WorldSettingsUi>,
}

#[derive(Default)]
enum Mode {
    #[default]
    List,
    WorldActions,
    BackupActions,
    Rename(String),
    DeleteWorld,
    RestoreBackup,
    DeleteBackup,
}

impl WorldsUi {
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
                *message = "Backup operation finished.".into();
                self.pending_operation = None;
                self.refresh_needed = true;
            }
            OperationStateDto::Failed | OperationStateDto::Cancelled => {
                *message = operation
                    .error
                    .map_or_else(|| "Backup operation failed.".into(), |error| error.message);
                self.pending_operation = None;
            }
            _ => {
                *message = operation
                    .status_line
                    .unwrap_or_else(|| "Backup operation is running…".into());
            }
        }
    }

    pub(super) async fn handle(
        &mut self,
        key: KeyCode,
        snapshot: &Snapshot,
        message: &mut String,
    ) -> bool {
        if let Some(settings) = &mut self.settings {
            if settings.handle(key, message).await {
                self.settings = None;
            }
            if message.starts_with("World settings saved") {
                self.refresh_needed = true;
            }
            return true;
        }
        let worlds = snapshot
            .worlds
            .as_ref()
            .map(|worlds| worlds.slots.as_slice())
            .unwrap_or(&[]);
        let backups = backups_for_active(snapshot);
        match &mut self.mode {
            Mode::Rename(draft) => {
                match key {
                    KeyCode::Esc => self.mode = Mode::List,
                    KeyCode::Backspace => {
                        draft.pop();
                    }
                    KeyCode::Char(c) if !c.is_control() => draft.push(c),
                    KeyCode::Enter => {
                        let name = draft.trim().to_string();
                        if name.is_empty() {
                            *message = "World name cannot be empty.".into();
                        } else if let Some(world) = worlds.get(self.world_index) {
                            let id = world.id.clone();
                            self.mode = Mode::List;
                            self.mutate_world(
                                "/v1/worlds/rename",
                                json!({"slotId":id,"name":name}),
                                "World renamed.",
                                message,
                            )
                            .await;
                        }
                    }
                    _ => {}
                }
                return true;
            }
            Mode::DeleteWorld | Mode::RestoreBackup | Mode::DeleteBackup => {
                let mode = std::mem::take(&mut self.mode);
                if matches!(
                    key,
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter
                ) {
                    match mode {
                        Mode::DeleteWorld => {
                            if let Some(world) = worlds.get(self.world_index) {
                                self.mutate_world(
                                    "/v1/worlds/delete",
                                    json!({"slotId":world.id}),
                                    "World deleted.",
                                    message,
                                )
                                .await;
                            }
                        }
                        Mode::RestoreBackup => {
                            if let Some(backup) = backups.get(self.backup_index) {
                                match SharedClient::connect_local().await {
                                    Ok(client) => match client
                                        .post_json::<_, BackupRestoreResultDto>(
                                            "/v1/backups/restore",
                                            &json!({"backupId":backup.id}),
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            self.pending_operation = result.operation_id;
                                            *message = "Backup restore started.".into();
                                            self.refresh_needed = true;
                                        }
                                        Err(error) => *message = error.to_string(),
                                    },
                                    Err(error) => *message = error.to_string(),
                                }
                            }
                        }
                        Mode::DeleteBackup => {
                            if let Some(backup) = backups.get(self.backup_index) {
                                match SharedClient::connect_local().await {
                                    Ok(client) => match client
                                        .post_json::<_, SimpleResultDto>(
                                            "/v1/backups/delete",
                                            &json!({"backupId":backup.id}),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            *message = "Backup deleted.".into();
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
                } else {
                    *message = "Action cancelled.".into();
                }
                return true;
            }
            Mode::WorldActions | Mode::BackupActions => {
                let is_world = matches!(self.mode, Mode::WorldActions);
                let action_count = if is_world {
                    WORLD_ACTIONS.len()
                } else {
                    BACKUP_ACTIONS.len()
                };
                match key {
                    KeyCode::Esc => self.mode = Mode::List,
                    KeyCode::Up => self.action_index = self.action_index.saturating_sub(1),
                    KeyCode::Down => {
                        self.action_index = (self.action_index + 1).min(action_count - 1)
                    }
                    KeyCode::Enter => {
                        if is_world {
                            match self.action_index {
                                0 => *message = "Convert World page is coming later.".into(),
                                1 => {
                                    if let Some(world) = worlds.get(self.world_index) {
                                        match WorldSettingsUi::load(&world.id, snapshot).await {
                                            Ok(settings) => {
                                                self.settings = Some(settings);
                                                self.mode = Mode::List;
                                            }
                                            Err(error) => *message = error,
                                        }
                                    }
                                }
                                2 => {
                                    if let Some(world) = worlds.get(self.world_index) {
                                        self.mode = Mode::Rename(world.name.clone());
                                    }
                                }
                                3 => {
                                    if let Some(world) = worlds.get(self.world_index) {
                                        self.mode = Mode::List;
                                        self.mutate_world(
                                            "/v1/worlds/duplicate",
                                            json!({"slotId":world.id}),
                                            "World duplicated.",
                                            message,
                                        )
                                        .await;
                                    }
                                }
                                4 => self.mode = Mode::DeleteWorld,
                                _ => {}
                            }
                        } else {
                            self.mode = if self.action_index == 0 {
                                Mode::RestoreBackup
                            } else {
                                Mode::DeleteBackup
                            };
                        }
                    }
                    _ => {}
                }
                return true;
            }
            Mode::List => {}
        }
        match key {
            KeyCode::Tab => {
                self.backup_focus = !self.backup_focus;
                true
            }
            KeyCode::Up => {
                if self.backup_focus {
                    self.backup_index = self.backup_index.saturating_sub(1);
                } else {
                    self.world_index = self.world_index.saturating_sub(1);
                }
                true
            }
            KeyCode::Down => {
                if self.backup_focus {
                    self.backup_index =
                        (self.backup_index + 1).min(backups.len().saturating_sub(1));
                } else {
                    self.world_index = (self.world_index + 1).min(worlds.len().saturating_sub(1));
                }
                true
            }
            KeyCode::Enter => {
                if self.backup_focus && !backups.is_empty() {
                    self.mode = Mode::BackupActions;
                    self.action_index = 0;
                } else if !self.backup_focus && !worlds.is_empty() {
                    self.mode = Mode::WorldActions;
                    self.action_index = 0;
                }
                true
            }
            KeyCode::Char('b') => {
                match SharedClient::connect_local().await {
                    Ok(client) => match client
                        .post_json::<_, BackupNowResultDto>("/v1/backups/now", &json!({}))
                        .await
                    {
                        Ok(result) => {
                            self.pending_operation = result.operation_id;
                            *message = "Backup started.".into();
                            self.refresh_needed = true;
                        }
                        Err(error) => *message = error.to_string(),
                    },
                    Err(error) => *message = error.to_string(),
                }
                true
            }
            KeyCode::Char('r') => {
                if let Some(config) = &snapshot.backup_config {
                    match SharedClient::connect_local().await {
                        Ok(client) => match client
                            .post_json::<_, BackupConfigUpdateResultDto>(
                                "/v1/backups/config",
                                &json!({"autoBackupEnabled":!config.auto_backup_enabled}),
                            )
                            .await
                        {
                            Ok(_) => {
                                *message = if config.auto_backup_enabled {
                                    "Recurring backups disabled."
                                } else {
                                    "Recurring backups enabled."
                                }
                                .into();
                                self.refresh_needed = true;
                            }
                            Err(error) => *message = error.to_string(),
                        },
                        Err(error) => *message = error.to_string(),
                    }
                } else {
                    *message = "Backup settings unavailable.".into();
                }
                true
            }
            KeyCode::Char('n') => {
                *message = "Create New World page is coming later.".into();
                true
            }
            _ => false,
        }
    }

    async fn mutate_world(
        &mut self,
        path: &str,
        body: serde_json::Value,
        success: &str,
        message: &mut String,
    ) {
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .post_json::<_, WorldMutationResultDto>(path, &body)
                .await
            {
                Ok(result) if result.success => {
                    *message = success.into();
                    self.refresh_needed = true;
                }
                Ok(result) => *message = result.message,
                Err(error) => *message = error.to_string(),
            },
            Err(error) => *message = error.to_string(),
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
        if let Some(settings) = &self.settings {
            return settings.draw(out, x, width, height);
        }
        let Some(worlds) = &snapshot.worlds else {
            paint(
                out,
                x,
                7,
                width,
                if snapshot.active_server_id.is_some() {
                    "Worlds unavailable."
                } else {
                    "Select or create a server to view worlds."
                },
                Color::DarkGrey,
            )?;
            return Ok(());
        };
        paint(out, x, 6, width, "WORLD SLOTS", Color::DarkGrey)?;
        paint(
            out,
            x,
            7,
            width,
            "World settings travel with each slot. Server settings apply to every world.",
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            9,
            width,
            &format!(
                "{} saved world{}   n New world",
                worlds.slots.len(),
                if worlds.slots.len() == 1 { "" } else { "s" }
            ),
            Color::White,
        )?;
        let backup_y = if height >= 30 {
            (height / 2).max(18)
        } else {
            height.saturating_sub(8)
        };
        let world_rows = backup_y.saturating_sub(13) as usize;
        if worlds.slots.is_empty() {
            paint(
                out,
                x + 2,
                11,
                width - 2,
                "No saved worlds.",
                Color::DarkGrey,
            )?;
        }
        for (index, world) in worlds.slots.iter().take(world_rows).enumerate() {
            let marker = if !self.backup_focus && index == self.world_index {
                ">"
            } else {
                " "
            };
            let active = if world.is_active { " · Active" } else { "" };
            paint(
                out,
                x + 2,
                11 + index as u16,
                width / 2,
                &format!("{marker} {}{active}", world.name),
                if index == self.world_index && !self.backup_focus {
                    Color::Green
                } else {
                    Color::White
                },
            )?;
        }
        if height >= 30 {
            paint(
                out,
                x,
                backup_y - 2,
                width,
                &"─".repeat(width as usize),
                Color::DarkGrey,
            )?;
            let active_name = worlds
                .slots
                .iter()
                .find(|world| world.is_active)
                .map_or("No active world", |world| world.name.as_str());
            paint(
                out,
                x,
                backup_y - 1,
                width,
                &format!("BACKUPS · {active_name}"),
                Color::DarkGrey,
            )?;
            let backups = backups_for_active(snapshot);
            let recurring = snapshot
                .backup_config
                .as_ref()
                .map_or("unavailable", |config| {
                    if config.auto_backup_enabled {
                        "on"
                    } else {
                        "off"
                    }
                });
            paint(
                out,
                x,
                backup_y,
                width,
                &format!(
                    "{} backup{}   b Back up now   r Recurring: {recurring}",
                    backups.len(),
                    if backups.len() == 1 { "" } else { "s" }
                ),
                Color::White,
            )?;
            if backups.is_empty() {
                paint(
                    out,
                    x + 2,
                    backup_y + 2,
                    width - 2,
                    "No backups for the active world yet.",
                    Color::DarkGrey,
                )?;
            }
            for (index, backup) in backups
                .iter()
                .take(height.saturating_sub(backup_y + 11) as usize)
                .enumerate()
            {
                let marker = if self.backup_focus && index == self.backup_index {
                    ">"
                } else {
                    " "
                };
                paint(
                    out,
                    x + 2,
                    backup_y + 2 + index as u16,
                    width - 4,
                    &format!("{marker} {}", backup.display_name),
                    if self.backup_focus && index == self.backup_index {
                        Color::Green
                    } else {
                        Color::White
                    },
                )?;
            }
        }
        let hint_y = height.saturating_sub(8);
        paint(
            out,
            x,
            hint_y,
            width,
            "Tab World/Backups   ↑↓ Choose   Enter Actions   b Backup now",
            Color::DarkGrey,
        )?;
        let menu_x = x + width / 2;
        let menu_y = 11;
        match &self.mode {
            Mode::WorldActions | Mode::BackupActions => {
                let actions: &[&str] = if matches!(self.mode, Mode::WorldActions) {
                    &WORLD_ACTIONS
                } else {
                    &BACKUP_ACTIONS
                };
                paint(
                    out,
                    menu_x,
                    menu_y - 2,
                    width / 2 - 2,
                    "ACTIONS · Esc Close",
                    Color::DarkGrey,
                )?;
                for (index, action) in actions.iter().enumerate() {
                    paint(
                        out,
                        menu_x,
                        menu_y + index as u16,
                        width / 2 - 2,
                        &format!(
                            "{} {action}",
                            if index == self.action_index { ">" } else { " " }
                        ),
                        if index == self.action_index {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                }
            }
            Mode::Rename(draft) => paint(
                out,
                menu_x,
                menu_y,
                width / 2 - 2,
                &format!("New name: {draft}_"),
                Color::Green,
            )?,
            Mode::DeleteWorld => paint(
                out,
                menu_x,
                menu_y,
                width / 2 - 2,
                "Delete this world? y/Enter Yes · Esc No",
                Color::Green,
            )?,
            Mode::RestoreBackup => paint(
                out,
                menu_x,
                menu_y,
                width / 2 - 2,
                "Restore this backup? y/Enter Yes · Esc No",
                Color::Green,
            )?,
            Mode::DeleteBackup => paint(
                out,
                menu_x,
                menu_y,
                width / 2 - 2,
                "Delete this backup? y/Enter Yes · Esc No",
                Color::Green,
            )?,
            Mode::List => {}
        }
        Ok(())
    }
}

fn backups_for_active(snapshot: &Snapshot) -> Vec<&msc_api::dto::BackupItemDto> {
    let active_id = snapshot
        .worlds
        .as_ref()
        .and_then(|worlds| worlds.active_slot_id.as_deref());
    snapshot.backups.as_ref().map_or_else(Vec::new, |response| {
        response
            .backups
            .iter()
            .filter(|backup| {
                backup.slot_id.as_deref().is_none() || backup.slot_id.as_deref() == active_id
            })
            .collect()
    })
}
