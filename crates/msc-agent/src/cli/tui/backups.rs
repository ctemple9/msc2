//! Backup context for the selected world slot.
//!
//! The agent owns archive verification and retention safety.  The terminal
//! presents the state the API actually reports and never guesses that an
//! archive is verified when the DTO does not provide that field.

use crossterm::event::KeyCode;
use msc_api::dto::{
    BackupConfigResponseDto, BackupConfigUpdateRequestDto, BackupConfigUpdateResultDto,
    BackupDeleteRequestDto, BackupNowResultDto, BackupRestoreRequestDto, BackupRestoreResultDto,
    BackupsResponseDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupMutation {
    Manual,
    Restore {
        backup_id: String,
    },
    Delete {
        backup_id: String,
    },
    UpdateConfig {
        enabled: Option<bool>,
        interval_minutes: Option<i64>,
        max_count: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupIntent {
    Confirm(BackupMutation),
}

#[derive(Debug, Clone, Default)]
pub struct BackupsState {
    pub response: Option<BackupsResponseDto>,
    pub config: Option<BackupConfigResponseDto>,
    pub loaded: bool,
    pub error: Option<String>,
    pub selected_backup: usize,
    pub context_slot_id: Option<String>,
    pub open: bool,
    pub input: Option<String>,
    pub status: Option<String>,
}

impl BackupsState {
    pub async fn load(
        client: &SharedClient,
        context_slot_id: Option<String>,
    ) -> Result<Self, CliError> {
        let response: BackupsResponseDto = client.get_json("/v1/backups").await?;
        let config: Option<BackupConfigResponseDto> =
            client.get_json("/v1/backups/config").await.ok();
        let mut state = Self {
            response: Some(response),
            config,
            loaded: true,
            open: true,
            context_slot_id,
            ..Self::default()
        };
        state.normalize_selection();
        Ok(state)
    }

    pub fn backups(&self) -> &[msc_api::dto::BackupItemDto] {
        self.response
            .as_ref()
            .map(|response| response.backups.as_slice())
            .unwrap_or_default()
    }

    pub fn visible_backups(&self) -> Vec<&msc_api::dto::BackupItemDto> {
        self.response
            .as_ref()
            .map(|response| {
                response
                    .backups
                    .iter()
                    .filter(|backup| {
                        self.context_slot_id
                            .as_ref()
                            .is_none_or(|slot_id| backup.slot_id.as_ref() == Some(slot_id))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn selected_backup(&self) -> Option<&msc_api::dto::BackupItemDto> {
        self.visible_backups().get(self.selected_backup).copied()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<BackupIntent> {
        if let Some(value) = self.input.take() {
            return self.handle_input(value, key);
        }
        match key {
            KeyCode::Esc | KeyCode::Char('b') => self.open = false,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('m') => return Some(BackupIntent::Confirm(BackupMutation::Manual)),
            KeyCode::Char('r') => {
                let backup_id = self.selected_backup()?.id.clone();
                return Some(BackupIntent::Confirm(BackupMutation::Restore { backup_id }));
            }
            KeyCode::Char('d') => {
                let backup_id = self.selected_backup()?.id.clone();
                return Some(BackupIntent::Confirm(BackupMutation::Delete { backup_id }));
            }
            KeyCode::Char('c') => self.input = Some(String::new()),
            KeyCode::Char('R') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn handle_input(&mut self, value: String, key: KeyCode) -> Option<BackupIntent> {
        let mut value = value;
        match key {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.input = Some(value);
            }
            KeyCode::Char(character) => {
                value.push(character);
                self.input = Some(value);
            }
            KeyCode::Enter => {
                let mut fields = value.split(',').map(str::trim);
                let enabled = fields.next()?.parse::<bool>().ok()?;
                let interval_minutes = fields.next()?.parse::<i64>().ok()?;
                let max_count = fields.next()?.parse::<i64>().ok()?;
                return Some(BackupIntent::Confirm(BackupMutation::UpdateConfig {
                    enabled: Some(enabled),
                    interval_minutes: Some(interval_minutes),
                    max_count: Some(max_count),
                }));
            }
            _ => self.input = Some(value),
        }
        None
    }

    fn move_selection(&mut self, offset: isize) {
        let count = self.visible_backups().len();
        if count > 0 {
            self.selected_backup =
                (self.selected_backup as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn normalize_selection(&mut self) {
        self.selected_backup = self
            .selected_backup
            .min(self.visible_backups().len().saturating_sub(1));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupMutationResult {
    pub message: String,
    pub operation_id: Option<String>,
}

pub async fn execute(
    client: &SharedClient,
    mutation: BackupMutation,
) -> Result<BackupMutationResult, CliError> {
    match mutation {
        BackupMutation::Manual => {
            let result: BackupNowResultDto = client
                .post_json("/v1/backups/now", &serde_json::json!({}))
                .await?;
            Ok(BackupMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            })
        }
        BackupMutation::Restore { backup_id } => {
            let result: BackupRestoreResultDto = client
                .post_json(
                    "/v1/backups/restore",
                    &BackupRestoreRequestDto { backup_id },
                )
                .await?;
            Ok(BackupMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            })
        }
        BackupMutation::Delete { backup_id } => {
            let result: msc_api::dto::SimpleResultDto = client
                .post_json("/v1/backups/delete", &BackupDeleteRequestDto { backup_id })
                .await?;
            Ok(BackupMutationResult {
                message: result.result,
                operation_id: None,
            })
        }
        BackupMutation::UpdateConfig {
            enabled,
            interval_minutes,
            max_count,
        } => {
            let result: BackupConfigUpdateResultDto = client
                .post_json(
                    "/v1/backups/config",
                    &BackupConfigUpdateRequestDto {
                        auto_backup_enabled: enabled,
                        auto_backup_interval_minutes: interval_minutes,
                        auto_backup_max_count: max_count,
                    },
                )
                .await?;
            if result.success {
                Ok(BackupMutationResult {
                    message: result.message,
                    operation_id: None,
                })
            } else {
                Err(CliError::usage(result.message))
            }
        }
    }
}
