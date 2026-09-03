//! Preferences and reset boundaries owned by the terminal client.
//!
//! Client reset clears only this process's local presentation state. Host
//! reset is a separate, authenticated request to the existing destructive
//! agent route and keeps its exact confirmation text and operation response.

use crossterm::event::KeyCode;
use msc_api::dto::HostResetAcceptedDto;

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppSettingsSurface {
    #[default]
    Preferences,
    HostReset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppSettingsIntent {
    ResetClient,
    HostReset { mode: String, confirmation: String },
}

#[derive(Debug, Clone, Default)]
pub struct AppSettingsState {
    pub surface: AppSettingsSurface,
    pub show_first_session_guide: bool,
    pub host_reset_mode: String,
    pub host_reset_confirmation: Option<String>,
    pub host_reset_result: Option<HostResetAcceptedDto>,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl AppSettingsState {
    pub fn handle_key(&mut self, key: KeyCode) -> Option<AppSettingsIntent> {
        if let Some(mut value) = self.host_reset_confirmation.take() {
            return match key {
                KeyCode::Esc => {
                    self.surface = AppSettingsSurface::Preferences;
                    None
                }
                KeyCode::Backspace => {
                    value.pop();
                    self.host_reset_confirmation = Some(value);
                    None
                }
                KeyCode::Char(character) => {
                    value.push(character);
                    self.host_reset_confirmation = Some(value);
                    None
                }
                KeyCode::Enter => Some(AppSettingsIntent::HostReset {
                    mode: self.host_reset_mode.clone(),
                    confirmation: value,
                }),
                _ => {
                    self.host_reset_confirmation = Some(value);
                    None
                }
            };
        }

        match key {
            KeyCode::Char('1') => self.show_first_session_guide = !self.show_first_session_guide,
            KeyCode::Char('c') => return Some(AppSettingsIntent::ResetClient),
            KeyCode::Char('2') => {
                self.surface = AppSettingsSurface::HostReset;
                self.host_reset_mode = "configuration".to_string();
                self.host_reset_confirmation = Some(String::new());
            }
            KeyCode::Char('3') => {
                self.surface = AppSettingsSurface::HostReset;
                self.host_reset_mode = "everything".to_string();
                self.host_reset_confirmation = Some(String::new());
            }
            KeyCode::Esc => self.surface = AppSettingsSurface::Preferences,
            _ => {}
        }
        None
    }

    pub async fn reset_host(
        client: &SharedClient,
        mode: String,
        confirmation: String,
    ) -> Result<HostResetAcceptedDto, CliError> {
        client
            .post_json(
                "/v1/host/reset",
                &serde_json::json!({ "mode": mode, "confirmation": confirmation }),
            )
            .await
    }
}
