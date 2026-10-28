//! Agent-provided server settings, shared by Java flavors and Bedrock.

use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{SettingFieldDto, SettingsUpdateResultDto};
use serde_json::json;

use super::{Snapshot, paint};
use crate::cli::transport::SharedClient;

#[derive(Default)]
pub(super) struct SettingsUi {
    selected: usize,
    scroll: usize,
    edit: Option<String>,
    choosing: Option<usize>,
    confirm: Option<(String, String)>,
    refresh_needed: bool,
}

fn fields(snapshot: &Snapshot) -> Vec<&SettingFieldDto> {
    snapshot
        .settings
        .as_ref()
        .map_or_else(Vec::new, |settings| {
            settings
                .sections
                .iter()
                .flat_map(|section| &section.fields)
                .collect()
        })
}

impl SettingsUi {
    pub(super) fn needs_refresh(&self) -> bool {
        self.refresh_needed
    }
    pub(super) fn refreshed(&mut self) {
        self.refresh_needed = false;
    }

    async fn save(&mut self, key: &str, value: &str, token: Option<&str>, message: &mut String) {
        let body = if let Some(token) = token {
            json!({"changes": {key: value}, "confirmation": token})
        } else {
            json!({"changes": {key: value}})
        };
        match SharedClient::connect_local().await {
            Ok(client) => match client
                .post_json::<_, SettingsUpdateResultDto>("/v1/settings", &body)
                .await
            {
                Ok(result) => {
                    *message = result
                        .rejected
                        .as_ref()
                        .and_then(|rejected| rejected.first())
                        .map_or(result.message, |rejection| rejection.reason.clone());
                    if result.restart_required {
                        message.push_str(" Restart the server to apply it.");
                    }
                    self.refresh_needed = true;
                }
                Err(error) => {
                    // The API returns its own acknowledgement token for settings
                    // that can change world behavior.
                    if let Some((token, explanation)) = error.settings_confirmation() {
                        self.confirm = Some((value.to_string(), token));
                        *message = format!("{explanation} Press y to confirm, Esc to cancel.");
                    } else {
                        *message = error.to_string();
                    }
                }
            },
            Err(error) => *message = error.to_string(),
        }
    }

    pub(super) async fn handle(
        &mut self,
        key: KeyCode,
        snapshot: &Snapshot,
        message: &mut String,
    ) -> bool {
        if snapshot.settings.is_none() {
            return false;
        }
        let all = fields(snapshot);
        self.selected = self.selected.min(all.len().saturating_sub(1));
        if let Some((value, token)) = self.confirm.take() {
            if matches!(key, KeyCode::Char('y') | KeyCode::Char('Y')) {
                if let Some(field) = all.get(self.selected) {
                    self.save(&field.key, &value, Some(&token), message).await;
                }
            } else {
                *message = "Setting change cancelled.".into();
            }
            return true;
        }
        if let Some(mut draft) = self.edit.take() {
            match key {
                KeyCode::Esc => *message = "Edit cancelled.".into(),
                KeyCode::Backspace => {
                    draft.pop();
                    self.edit = Some(draft);
                }
                KeyCode::Char(c) => {
                    draft.push(c);
                    self.edit = Some(draft);
                }
                KeyCode::Enter => {
                    if let Some(field) = all.get(self.selected) {
                        if let Some(limit) = field.max_length
                            && draft.chars().count() > limit as usize
                        {
                            *message =
                                format!("{} must be at most {limit} characters.", field.label);
                            self.edit = Some(draft);
                        } else if field.r#type == "int"
                            && draft.parse::<i64>().ok().is_none_or(|number| {
                                field.min_int.is_some_and(|min| number < min)
                                    || field.max_int.is_some_and(|max| number > max)
                            })
                        {
                            *message =
                                format!("{} needs a number within its allowed range.", field.label);
                            self.edit = Some(draft);
                        } else {
                            self.save(&field.key, &draft, None, message).await;
                        }
                    }
                }
                _ => self.edit = Some(draft),
            }
            return true;
        }
        if let Some(mut choice) = self.choosing.take() {
            if let Some(field) = all.get(self.selected)
                && let Some(options) = &field.options
            {
                match key {
                    KeyCode::Esc => *message = "Choice cancelled.".into(),
                    KeyCode::Up | KeyCode::Left => {
                        choice = choice.saturating_sub(1);
                        self.choosing = Some(choice);
                    }
                    KeyCode::Down | KeyCode::Right => {
                        choice = (choice + 1).min(options.len().saturating_sub(1));
                        self.choosing = Some(choice);
                    }
                    KeyCode::Enter => {
                        if let Some(option) = options.get(choice) {
                            self.save(&field.key, &option.value, None, message).await;
                        }
                    }
                    _ => self.choosing = Some(choice),
                }
            }
            return true;
        }
        match key {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(all.len().saturating_sub(1)),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(8),
            KeyCode::PageDown => {
                self.selected = (self.selected + 8).min(all.len().saturating_sub(1))
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('e') => {
                if let Some(field) = all.get(self.selected) {
                    if !snapshot
                        .settings
                        .as_ref()
                        .is_some_and(|settings| settings.editable)
                    {
                        *message = "Settings are read-only for this server.".into();
                    } else {
                        match field.r#type.as_str() {
                            "bool" => {
                                self.save(
                                    &field.key,
                                    if field.value == "true" {
                                        "false"
                                    } else {
                                        "true"
                                    },
                                    None,
                                    message,
                                )
                                .await
                            }
                            "enum" => {
                                self.choosing = Some(
                                    field
                                        .options
                                        .as_ref()
                                        .and_then(|options| {
                                            options
                                                .iter()
                                                .position(|option| option.value == field.value)
                                        })
                                        .unwrap_or(0),
                                )
                            }
                            _ => self.edit = Some(field.value.clone()),
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn draw(
        &self,
        out: &mut Stdout,
        snapshot: &Snapshot,
        x: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        let Some(settings) = &snapshot.settings else {
            return paint(
                out,
                x,
                7,
                width,
                "Select a server to view its settings.",
                Color::DarkGrey,
            );
        };
        paint(
            out,
            x,
            6,
            width,
            &format!("SERVER PROPERTIES · {}", settings.server_name),
            Color::DarkGrey,
        )?;
        if let Some(note) = &settings.note {
            paint(out, x, 7, width, note, Color::DarkGrey)?;
        }
        let mut rows = Vec::new();
        let mut field_index = 0usize;
        for section in &settings.sections {
            rows.push((None, section.title.to_uppercase()));
            for field in &section.fields {
                let value = if field.r#type == "bool" {
                    if field.value == "true" {
                        "On".to_string()
                    } else {
                        "Off".to_string()
                    }
                } else {
                    field
                        .options
                        .as_ref()
                        .and_then(|options| {
                            options.iter().find(|option| option.value == field.value)
                        })
                        .map_or_else(|| field.value.clone(), |option| option.label.clone())
                };
                let value = if self.selected == field_index {
                    self.edit.clone().unwrap_or(value)
                } else {
                    value
                };
                rows.push((
                    Some(field_index),
                    format!(
                        "{:<30} {}{}",
                        field.label,
                        value,
                        field
                            .unit
                            .as_ref()
                            .map_or(String::new(), |unit| format!(" {unit}"))
                    ),
                ));
                field_index += 1;
            }
            rows.push((None, String::new()));
        }
        let available = height.saturating_sub(16) as usize;
        let selected_row = rows
            .iter()
            .position(|(index, _)| *index == Some(self.selected))
            .unwrap_or(0);
        let scroll = self
            .scroll
            .max(selected_row.saturating_sub(available.saturating_sub(1)))
            .min(selected_row);
        for (line, (index, content)) in rows.iter().skip(scroll).take(available).enumerate() {
            let selected = *index == Some(self.selected);
            let color = if selected {
                Color::Green
            } else if index.is_none() {
                Color::DarkGrey
            } else {
                Color::White
            };
            paint(
                out,
                x,
                9 + line as u16,
                width,
                &format!("{} {content}", if selected { ">" } else { " " }),
                color,
            )?;
        }
        if let Some(choice) = self.choosing
            && let Some(field) = fields(snapshot).get(self.selected)
            && let Some(options) = &field.options
        {
            paint(
                out,
                x,
                height - 8,
                width,
                &format!(
                    "{}: {}",
                    field.label,
                    options
                        .get(choice)
                        .map_or("", |option| option.label.as_str())
                ),
                Color::Green,
            )?;
        }
        paint(
            out,
            x,
            height - 7,
            width,
            "↑↓ Select  PgUp/PgDn Scroll  Enter Edit/Toggle  Esc Cancel",
            Color::DarkGrey,
        )?;
        Ok(())
    }
}
