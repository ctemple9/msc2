//! Slot-local world profile editor.

use std::collections::BTreeMap;
use std::io::{self, Stdout};

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{
    CapabilitiesResponseDto, WorldSettingsCapabilitiesDto, WorldSlotWithProfileDto,
};
use serde_json::{Value, json};

use super::{Snapshot, paint};
use crate::cli::transport::{SharedClient, encode_query_component};

struct Field {
    key: &'static str,
    label: &'static str,
    group: &'static str,
    choices: &'static [&'static str],
}
const FIELDS: &[Field] = &[
    Field {
        key: "identity.name",
        label: "World Name",
        group: "ESSENTIALS",
        choices: &[],
    },
    Field {
        key: "identity.seed",
        label: "Seed",
        group: "ESSENTIALS",
        choices: &[],
    },
    Field {
        key: "gameplay.difficulty",
        label: "Difficulty",
        group: "ESSENTIALS",
        choices: &["peaceful", "easy", "normal", "hard"],
    },
    Field {
        key: "gameplay.default-game-mode",
        label: "Default Game Mode",
        group: "ESSENTIALS",
        choices: &["survival", "creative", "adventure", "spectator"],
    },
    Field {
        key: "identity.level-name",
        label: "Minecraft Folder",
        group: "WORLD GENERATION",
        choices: &[],
    },
    Field {
        key: "generation.world-type",
        label: "World Type",
        group: "WORLD GENERATION",
        choices: &["default", "flat"],
    },
    Field {
        key: "generation.flat-preset",
        label: "Flat Preset",
        group: "WORLD GENERATION",
        choices: &[],
    },
    Field {
        key: "generation.biome-source",
        label: "Biome Source",
        group: "WORLD GENERATION",
        choices: &[],
    },
    Field {
        key: "generation.structures",
        label: "Generate Structures",
        group: "WORLD GENERATION",
        choices: &["false", "true"],
    },
    Field {
        key: "generation.bonus-chest",
        label: "Bonus Chest",
        group: "WORLD GENERATION",
        choices: &["false", "true"],
    },
    Field {
        key: "generation.generator-options",
        label: "Generator Options",
        group: "WORLD GENERATION",
        choices: &[],
    },
    Field {
        key: "gameplay.hardcore",
        label: "Hardcore",
        group: "GAMEPLAY RULES",
        choices: &["false", "true"],
    },
    Field {
        key: "gameplay.commands",
        label: "Allow Commands",
        group: "GAMEPLAY RULES",
        choices: &["false", "true"],
    },
    Field {
        key: "gameplay.cheats",
        label: "Cheats",
        group: "GAMEPLAY RULES",
        choices: &["false", "true"],
    },
    Field {
        key: "gameplay.coordinates",
        label: "Show Coordinates",
        group: "GAMEPLAY RULES",
        choices: &["false", "true"],
    },
    Field {
        key: "gameplay.starting-map",
        label: "Starting Map",
        group: "GAMEPLAY RULES",
        choices: &["false", "true"],
    },
];

pub(super) struct WorldSettingsUi {
    slot: WorldSlotWithProfileDto,
    capabilities: Option<WorldSettingsCapabilitiesDto>,
    selected: usize,
    scroll: usize,
    edit: Option<String>,
    pending: BTreeMap<String, Value>,
    confirm: Option<String>,
}

impl WorldSettingsUi {
    pub(super) async fn load(slot_id: &str, snapshot: &Snapshot) -> Result<Self, String> {
        let client = SharedClient::connect_local()
            .await
            .map_err(|error| error.to_string())?;
        let slot = client
            .get_json::<WorldSlotWithProfileDto>(&format!(
                "/v1/worlds/{}/profile",
                encode_query_component(slot_id)
            ))
            .await
            .map_err(|error| error.to_string())?;
        let server = snapshot.active_server_id.as_ref().and_then(|id| {
            snapshot
                .servers
                .as_ref()?
                .iter()
                .find(|server| &server.id == id)
        });
        let mut query = format!(
            "serverType={}",
            server.map_or("java", |server| server.server_type.as_str())
        );
        if let Some(version) = snapshot
            .components
            .as_ref()
            .and_then(|components| components.components.first())
            .and_then(|component| component.installed_version.as_deref())
        {
            query.push_str("&minecraftVersion=");
            query.push_str(&encode_query_component(version));
        }
        if let Some(flavor) = server.and_then(|server| server.java_flavor.as_deref()) {
            query.push_str("&javaFlavor=");
            query.push_str(&encode_query_component(flavor));
        }
        let capabilities = client
            .get_json::<CapabilitiesResponseDto>(&format!("/v1/capabilities?{query}"))
            .await
            .ok()
            .and_then(|response| response.world_settings);
        Ok(Self {
            slot,
            capabilities,
            selected: 0,
            scroll: 0,
            edit: None,
            pending: BTreeMap::new(),
            confirm: None,
        })
    }

    fn available(&self, field: &Field) -> bool {
        self.slot
            .profile
            .field_metadata
            .get(field.key)
            .is_some_and(|meta| meta.value_state != "unsupported")
            && self.capabilities.as_ref().is_none_or(|caps| {
                caps.fields
                    .get(field.key)
                    .is_some_and(|cap| cap.state != "unsupported")
            })
    }
    fn visible(&self) -> Vec<&Field> {
        FIELDS
            .iter()
            .filter(|field| self.available(field))
            .collect()
    }
    fn value(&self, field: &Field) -> Value {
        if let Some(value) = self.pending.get(field.key) {
            return value.clone();
        }
        let profile = serde_json::to_value(&self.slot.profile).unwrap_or(Value::Null);
        let mut parts = field.key.split('.');
        let section = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        let camel = name
            .split('-')
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part.to_string()
                } else {
                    let mut chars = part.chars();
                    chars.next().map_or(String::new(), |first| {
                        first.to_ascii_uppercase().to_string() + chars.as_str()
                    })
                }
            })
            .collect::<String>();
        profile
            .get(section)
            .and_then(|part| part.get(&camel))
            .cloned()
            .unwrap_or(Value::Null)
    }
    fn display(&self, field: &Field) -> String {
        let value = self.value(field);
        match value {
            Value::String(text) => {
                if text.is_empty() {
                    "—".into()
                } else {
                    text
                }
            }
            Value::Bool(value) => {
                if value {
                    "On".into()
                } else {
                    "Off".into()
                }
            }
            Value::Null => "—".into(),
            other => other.to_string(),
        }
    }
    async fn save(&mut self, message: &mut String) -> bool {
        if self.pending.is_empty() {
            *message = "No world settings changed.".into();
            return false;
        }
        let Ok(client) = SharedClient::connect_local().await else {
            *message = "Agent unavailable.".into();
            return false;
        };
        let mut body = json!({"changes": self.pending});
        if let Some(token) = self.confirm.as_deref() {
            body["confirmation"] = json!(token);
        }
        let route = format!(
            "/v1/worlds/{}/profile",
            encode_query_component(&self.slot.slot.id)
        );
        match client.post_json::<_, Value>(&route, &body).await {
            Ok(result) => {
                if result["success"].as_bool() != Some(true) {
                    *message = result["message"]
                        .as_str()
                        .unwrap_or("World settings were not saved.")
                        .into();
                    return false;
                }
                *message = format!(
                    "World settings saved ({})",
                    result["status"].as_str().unwrap_or("saved")
                );
                self.pending.clear();
                self.confirm = None;
                if let Ok(slot) = serde_json::from_value(result["slot"].clone()) {
                    self.slot = slot;
                }
                true
            }
            Err(error) => {
                if let Some((token, explanation)) = error.settings_confirmation() {
                    self.confirm = Some(token);
                    *message = format!("{explanation} Press y to confirm, Esc to cancel.");
                } else {
                    *message = error.to_string();
                }
                false
            }
        }
    }
    pub(super) async fn handle(&mut self, key: KeyCode, message: &mut String) -> bool {
        if self.confirm.is_some() {
            if matches!(key, KeyCode::Char('y') | KeyCode::Char('Y')) {
                return self.save(message).await;
            }
            self.confirm = None;
            *message = "Save cancelled.".into();
            return false;
        }
        if let Some(mut draft) = self.edit.take() {
            match key {
                KeyCode::Esc => {}
                KeyCode::Backspace => {
                    draft.pop();
                    self.edit = Some(draft);
                }
                KeyCode::Char(c) if !c.is_control() => {
                    draft.push(c);
                    self.edit = Some(draft);
                }
                KeyCode::Enter => {
                    if let Some(field) = self.visible().get(self.selected) {
                        let value = if draft.trim().is_empty() {
                            Value::Null
                        } else {
                            Value::String(draft)
                        };
                        self.pending.insert(field.key.into(), value);
                    }
                }
                _ => self.edit = Some(draft),
            }
            return false;
        }
        let visible = self.visible();
        match key {
            KeyCode::Esc => return true,
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(visible.len().saturating_sub(1))
            }
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(8),
            KeyCode::PageDown => {
                self.selected = (self.selected + 8).min(visible.len().saturating_sub(1))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.save(message).await;
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Enter
            | KeyCode::Char(' ')
            | KeyCode::Char('e') => {
                if let Some(field) = visible.get(self.selected) {
                    if field.choices.is_empty() {
                        self.edit = Some(self.value(field).as_str().unwrap_or("").to_string());
                    } else {
                        let current = self.value(field);
                        let index = field
                            .choices
                            .iter()
                            .position(|choice| current == *choice || current == (*choice == "true"))
                            .unwrap_or(0);
                        let next = if matches!(key, KeyCode::Left) {
                            index.checked_sub(1).unwrap_or(field.choices.len() - 1)
                        } else {
                            (index + 1) % field.choices.len()
                        };
                        let choice = field.choices[next];
                        self.pending.insert(
                            field.key.into(),
                            if choice == "true" {
                                json!(true)
                            } else if choice == "false" {
                                json!(false)
                            } else {
                                json!(choice)
                            },
                        );
                    }
                }
            }
            _ => {}
        }
        false
    }
    pub(super) fn draw(&self, out: &mut Stdout, x: u16, width: u16, height: u16) -> io::Result<()> {
        paint(
            out,
            x,
            6,
            width,
            &format!("WORLD SETTINGS — {}", self.slot.slot.name),
            Color::White,
        )?;
        paint(
            out,
            x,
            7,
            width,
            "Saved with this world. Server ports and player limits apply to every world.",
            Color::DarkGrey,
        )?;
        let visible = self.visible();
        let available = height.saturating_sub(17) as usize;
        let scroll = self
            .scroll
            .max(self.selected.saturating_sub(available.saturating_sub(1)))
            .min(self.selected);
        for (line, field) in visible.iter().enumerate().skip(scroll).take(available) {
            let y = 9 + (line - scroll) as u16;
            let value = if self.selected == line {
                self.edit.clone().unwrap_or_else(|| self.display(field))
            } else {
                self.display(field)
            };
            let row = format!(
                "{} {:<18} {:<25} {}{}",
                if self.selected == line { ">" } else { " " },
                field.group,
                field.label,
                value,
                if self.pending.contains_key(field.key) {
                    " *"
                } else {
                    ""
                }
            );
            paint(
                out,
                x + 2,
                y,
                width.saturating_sub(2),
                &row,
                if self.selected == line {
                    Color::Green
                } else {
                    Color::White
                },
            )?;
        }
        paint(
            out,
            x,
            height - 8,
            width,
            &format!("{} pending change(s)", self.pending.len()),
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            height - 7,
            width,
            "↑↓ Select  ←→ Change  Enter/e Edit  s Save  Esc Close",
            Color::DarkGrey,
        )?;
        Ok(())
    }
}
