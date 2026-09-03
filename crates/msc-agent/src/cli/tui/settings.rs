//! Keyboard-first server settings backed by the agent's schema response.
//!
//! The TUI does not know server.properties policy. It displays the sections,
//! values, bounds, options, and help identifiers supplied by the agent and
//! sends sparse changes back through `POST /v1/settings`.

use std::collections::HashMap;

use crossterm::event::KeyCode;
use msc_api::dto::{
    SettingFieldDto, SettingsResponseDto, SettingsUpdateRequestDto, SettingsUpdateResultDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsMutation {
    Update { changes: HashMap<String, String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsIntent {
    Confirm(SettingsMutation),
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub response: Option<SettingsResponseDto>,
    pub selected_section: usize,
    pub selected_field: usize,
    pub input: Option<(String, String)>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl SettingsState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let response = client.get_json("/v1/settings").await?;
        Ok(Self {
            response: Some(response),
            loaded: true,
            ..Self::default()
        })
    }

    pub fn selected_field(&self) -> Option<&SettingFieldDto> {
        self.response
            .as_ref()?
            .sections
            .get(self.selected_section)?
            .fields
            .get(self.selected_field)
    }

    pub fn select_section(&mut self, section: usize) {
        if self
            .response
            .as_ref()
            .is_some_and(|response| section < response.sections.len())
        {
            self.selected_section = section;
            self.selected_field = 0;
            self.input = None;
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<SettingsIntent> {
        if let Some((key_name, value)) = self.input.take() {
            return self.handle_input(key_name, value, key);
        }
        match key {
            KeyCode::Char('1'..='9') => {
                let section = match key {
                    KeyCode::Char(value) => value.to_digit(10).unwrap_or(1) as usize - 1,
                    _ => unreachable!(),
                };
                self.select_section(section);
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_field(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_field(-1),
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(field) = self.selected_field()
                    && self
                        .response
                        .as_ref()
                        .is_some_and(|response| response.editable)
                {
                    self.input = Some((field.key.clone(), field.value.clone()));
                }
            }
            KeyCode::Char(' ') => {
                let field = self.selected_field()?;
                if field.r#type.eq_ignore_ascii_case("boolean")
                    && self
                        .response
                        .as_ref()
                        .is_some_and(|response| response.editable)
                {
                    let value = if is_true(&field.value) {
                        "false"
                    } else {
                        "true"
                    };
                    return Some(SettingsIntent::Confirm(SettingsMutation::Update {
                        changes: HashMap::from([(field.key.clone(), value.to_string())]),
                    }));
                }
            }
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn move_field(&mut self, offset: isize) {
        let Some(section) = self
            .response
            .as_ref()
            .and_then(|response| response.sections.get(self.selected_section))
        else {
            return;
        };
        if section.fields.is_empty() {
            self.selected_field = 0;
            return;
        }
        self.selected_field = (self.selected_field as isize + offset)
            .rem_euclid(section.fields.len() as isize) as usize;
    }

    fn handle_input(
        &mut self,
        key_name: String,
        mut value: String,
        key: KeyCode,
    ) -> Option<SettingsIntent> {
        match key {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                self.input = Some((key_name, {
                    value.pop();
                    value
                }))
            }
            KeyCode::Char(character) => {
                self.input = Some((key_name, {
                    value.push(character);
                    value
                }));
            }
            KeyCode::Enter => {
                let key_name = key_name.trim().to_string();
                if !key_name.is_empty() {
                    return Some(SettingsIntent::Confirm(SettingsMutation::Update {
                        changes: HashMap::from([(key_name, value)]),
                    }));
                }
            }
            _ => self.input = Some((key_name, value)),
        }
        None
    }

    pub async fn update(
        client: &SharedClient,
        changes: HashMap<String, String>,
    ) -> Result<SettingsUpdateResultDto, CliError> {
        client
            .post_json("/v1/settings", &SettingsUpdateRequestDto { changes })
            .await
    }
}

fn is_true(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1"
    )
}
