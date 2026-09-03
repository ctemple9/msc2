//! Keyboard-first world-slot state for the Worlds and Backups vertical slice.
//!
//! A world slot is the parent context for its backups.  The TUI keeps that
//! relationship in its selection state and sends every mutation to the
//! existing authenticated world route; it does not maintain a second world
//! store locally.

use std::path::PathBuf;

use crossterm::event::KeyCode;
use msc_api::dto::{
    StagedUploadBeginRequestDto, StagedUploadCompleteResultDto, StagedUploadPurposeDto,
    WorldActivateRequestDto, WorldActivateResultDto, WorldConvertRequestDto, WorldConvertResultDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldImportRequestDto, WorldMutationResultDto,
    WorldRenameActiveWorldRequestDto, WorldRenameRequestDto, WorldRepairRequestDto,
    WorldRepairResultDto, WorldReplaceActiveRequestDto, WorldReplaceActiveResultDto,
    WorldReplaceRequestDto, WorldSlotsResponseDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldInputKind {
    Create,
    Rename,
    RenameActive,
    Copy,
    Import,
    ReplaceActive,
    Convert,
    Export,
}

impl WorldInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Create => "New world name",
            Self::Rename => "New slot name",
            Self::RenameActive => "New active world level name",
            Self::Copy => "Source slot id",
            Self::Import => "ZIP path | new slot name",
            Self::ReplaceActive => "Level name [| staged upload id]",
            Self::Convert => "target server id | format | target name OR target slot id",
            Self::Export => "Local ZIP output path",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldMutation {
    Create {
        name: String,
    },
    Rename {
        slot_id: String,
        name: String,
    },
    RenameActive {
        name: String,
    },
    Delete {
        slot_id: String,
    },
    Duplicate {
        slot_id: String,
    },
    Copy {
        destination_slot_id: String,
        source_slot_id: String,
    },
    SaveCurrent {
        slot_id: String,
    },
    Activate {
        slot_id: String,
    },
    ReplaceActive {
        level_name: String,
        staged_upload_id: Option<String>,
    },
    Export {
        slot_id: String,
        output: PathBuf,
    },
    Import {
        path: PathBuf,
        name: String,
    },
    Convert {
        source_slot_id: String,
        target_server_id: String,
        target_format: String,
        target_name: Option<String>,
        target_slot_id: Option<String>,
    },
    Repair {
        slot_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldIntent {
    OpenBackups,
    Confirm(WorldMutation),
}

#[derive(Debug, Clone, Default)]
pub struct WorldsState {
    pub response: Option<WorldSlotsResponseDto>,
    pub loaded: bool,
    pub error: Option<String>,
    pub selected_slot: usize,
    pub detail_open: bool,
    pub input: Option<(WorldInputKind, String)>,
    pub status: Option<String>,
}

impl WorldsState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let response: WorldSlotsResponseDto = client.get_json("/v1/worlds").await?;
        let mut state = Self {
            response: Some(response),
            loaded: true,
            ..Self::default()
        };
        state.normalize_selection();
        Ok(state)
    }

    pub fn slots(&self) -> &[msc_api::dto::WorldSlotDto] {
        self.response
            .as_ref()
            .map(|response| response.slots.as_slice())
            .unwrap_or_default()
    }

    pub fn active_slot(&self) -> Option<&msc_api::dto::WorldSlotDto> {
        let response = self.response.as_ref()?;
        response
            .active_slot_id
            .as_ref()
            .and_then(|id| response.slots.iter().find(|slot| &slot.id == id))
            .or_else(|| response.slots.iter().find(|slot| slot.is_active))
    }

    pub fn selected_slot(&self) -> Option<&msc_api::dto::WorldSlotDto> {
        self.slots().get(self.selected_slot)
    }

    pub fn selected_slot_id(&self) -> Option<&str> {
        self.selected_slot().map(|slot| slot.id.as_str())
    }

    pub fn selected_slot_name(&self) -> Option<&str> {
        self.selected_slot().map(|slot| slot.name.as_str())
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<WorldIntent> {
        if self.input.is_some() {
            return self.handle_input(key);
        }
        if self.detail_open {
            return self.handle_detail_key(key);
        }
        match key {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Enter => self.detail_open = self.selected_slot().is_some(),
            KeyCode::Char('c') => self.begin_input(WorldInputKind::Create),
            KeyCode::Char('i') => self.begin_input(WorldInputKind::Import),
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn handle_detail_key(&mut self, key: KeyCode) -> Option<WorldIntent> {
        let slot = self.selected_slot()?.clone();
        match key {
            KeyCode::Esc => self.detail_open = false,
            KeyCode::Char('b') => return Some(WorldIntent::OpenBackups),
            KeyCode::Char('n') => self.begin_input(WorldInputKind::Rename),
            KeyCode::Char('d') => {
                return Some(WorldIntent::Confirm(WorldMutation::Delete {
                    slot_id: slot.id,
                }));
            }
            KeyCode::Char('u') => {
                return Some(WorldIntent::Confirm(WorldMutation::Duplicate {
                    slot_id: slot.id,
                }));
            }
            KeyCode::Char('s') => {
                return Some(WorldIntent::Confirm(WorldMutation::SaveCurrent {
                    slot_id: slot.id,
                }));
            }
            KeyCode::Char('a') => {
                return Some(WorldIntent::Confirm(WorldMutation::Activate {
                    slot_id: slot.id,
                }));
            }
            KeyCode::Char('p') => {
                self.begin_input(WorldInputKind::Copy);
            }
            KeyCode::Char('N') => self.begin_input(WorldInputKind::RenameActive),
            KeyCode::Char('l') => self.begin_input(WorldInputKind::ReplaceActive),
            KeyCode::Char('e') => self.begin_input(WorldInputKind::Export),
            KeyCode::Char('v') => self.begin_input(WorldInputKind::Convert),
            KeyCode::Char('R') => {
                return Some(WorldIntent::Confirm(WorldMutation::Repair {
                    slot_id: slot.id,
                }));
            }
            _ => {}
        }
        None
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<WorldIntent> {
        let (kind, mut value) = self.input.take()?;
        match key {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.input = Some((kind, value));
            }
            KeyCode::Enter => return self.finish_input(kind, value),
            KeyCode::Char(character) => {
                value.push(character);
                self.input = Some((kind, value));
            }
            _ => self.input = Some((kind, value)),
        }
        None
    }

    fn finish_input(&mut self, kind: WorldInputKind, value: String) -> Option<WorldIntent> {
        let value = value.trim().to_string();
        let mutation = match kind {
            WorldInputKind::Create if !value.is_empty() => WorldMutation::Create { name: value },
            WorldInputKind::Rename if !value.is_empty() => WorldMutation::Rename {
                slot_id: self.selected_slot()?.id.clone(),
                name: value,
            },
            WorldInputKind::RenameActive if !value.is_empty() => {
                WorldMutation::RenameActive { name: value }
            }
            WorldInputKind::Copy if !value.is_empty() => WorldMutation::Copy {
                destination_slot_id: self.selected_slot()?.id.clone(),
                source_slot_id: value,
            },
            WorldInputKind::Import => {
                let (path, name) = value.split_once('|')?;
                WorldMutation::Import {
                    path: PathBuf::from(path.trim()),
                    name: name.trim().to_string(),
                }
            }
            WorldInputKind::ReplaceActive => {
                let mut parts = value.split('|').map(str::trim);
                let level_name = parts.next()?.to_string();
                let staged_upload_id = parts
                    .next()
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                WorldMutation::ReplaceActive {
                    level_name,
                    staged_upload_id,
                }
            }
            WorldInputKind::Export if !value.is_empty() => WorldMutation::Export {
                slot_id: self.selected_slot()?.id.clone(),
                output: PathBuf::from(value),
            },
            WorldInputKind::Convert => {
                let mut parts = value.split('|').map(str::trim);
                let target_server_id = parts.next()?.to_string();
                let target_format = parts.next()?.to_string();
                let destination = parts.next()?.to_string();
                let (target_name, target_slot_id) = if destination.starts_with("slot:") {
                    (
                        None,
                        Some(destination.trim_start_matches("slot:").trim().to_string()),
                    )
                } else {
                    (Some(destination), None)
                };
                WorldMutation::Convert {
                    source_slot_id: self.selected_slot()?.id.clone(),
                    target_server_id,
                    target_format,
                    target_name,
                    target_slot_id,
                }
            }
            _ => return None,
        };
        Some(WorldIntent::Confirm(mutation))
    }

    fn begin_input(&mut self, kind: WorldInputKind) {
        self.input = Some((kind, String::new()));
    }

    fn move_selection(&mut self, offset: isize) {
        let count = self.slots().len();
        if count > 0 {
            self.selected_slot =
                (self.selected_slot as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn normalize_selection(&mut self) {
        self.selected_slot = self.selected_slot.min(self.slots().len().saturating_sub(1));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMutationResult {
    pub message: String,
    pub operation_id: Option<String>,
}

pub async fn execute(
    client: &SharedClient,
    mutation: WorldMutation,
) -> Result<WorldMutationResult, CliError> {
    let result = match mutation {
        WorldMutation::Create { name } => {
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/create",
                    &WorldCreateRequestDto { name, seed: None },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Rename { slot_id, name } => {
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/rename",
                    &WorldRenameRequestDto { slot_id, name },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::RenameActive { name } => {
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/rename-active-world",
                    &WorldRenameActiveWorldRequestDto { name },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Delete { slot_id } => {
            let result: WorldMutationResultDto = client
                .post_json("/v1/worlds/delete", &WorldDeleteRequestDto { slot_id })
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Duplicate { slot_id } => {
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/duplicate",
                    &WorldDuplicateRequestDto { slot_id },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Copy {
            destination_slot_id,
            source_slot_id,
        } => {
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/replace",
                    &WorldReplaceRequestDto {
                        slot_id: destination_slot_id,
                        source_slot_id,
                    },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::SaveCurrent { .. } => {
            let result: WorldMutationResultDto = client
                .post_json("/v1/worlds/update", &serde_json::json!({}))
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Activate { slot_id } => {
            let result: WorldActivateResultDto = client
                .post_json("/v1/worlds/activate", &WorldActivateRequestDto { slot_id })
                .await?;
            WorldMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            }
        }
        WorldMutation::ReplaceActive {
            level_name,
            staged_upload_id,
        } => {
            let result: WorldReplaceActiveResultDto = client
                .post_json(
                    "/v1/worlds/replace-active-world",
                    &WorldReplaceActiveRequestDto {
                        new_level_name: level_name,
                        staged_upload_id,
                    },
                )
                .await?;
            WorldMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            }
        }
        WorldMutation::Export { slot_id, output } => {
            let result: WorldExportResultDto = client
                .post_json("/v1/worlds/export", &WorldExportRequestDto { slot_id })
                .await?;
            let bytes = client
                .get_raw_bytes(&format!(
                    "/v1/staged-downloads/{}",
                    result.staged_download_id
                ))
                .await?;
            tokio::fs::write(&output, &bytes).await.map_err(|error| {
                CliError::internal(format!("failed to write {}: {error}", output.display()))
            })?;
            WorldMutationResult {
                message: format!("Exported {} bytes to {}", bytes.len(), output.display()),
                operation_id: None,
            }
        }
        WorldMutation::Import { path, name } => {
            let bytes = tokio::fs::read(&path).await.map_err(|error| {
                CliError::usage(format!("failed to read {}: {error}", path.display()))
            })?;
            let begin: StagedUploadBeginRequestDto = StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldImport,
                content_type: Some("application/zip".to_string()),
                operation_id: None,
                file_id: None,
            };
            let staged: msc_api::dto::StagedUploadBeginResultDto =
                client.post_json("/v1/staged-uploads", &begin).await?;
            let _: StagedUploadCompleteResultDto = client
                .put_bytes(&staged.upload_path, "application/zip", bytes)
                .await?;
            let result: WorldMutationResultDto = client
                .post_json(
                    "/v1/worlds/import",
                    &WorldImportRequestDto {
                        name,
                        staged_upload_id: staged.staged_upload_id,
                        backup_id: None,
                    },
                )
                .await?;
            return mutation_result(result.success, result.message, None);
        }
        WorldMutation::Convert {
            source_slot_id,
            target_server_id,
            target_format,
            target_name,
            target_slot_id,
        } => {
            let result: WorldConvertResultDto = client
                .post_json(
                    "/v1/worlds/convert",
                    &WorldConvertRequestDto {
                        source_slot_id,
                        target_server_id,
                        target_format,
                        target_name,
                        target_slot_id,
                    },
                )
                .await?;
            WorldMutationResult {
                message: result.result,
                operation_id: Some(result.operation_id),
            }
        }
        WorldMutation::Repair { slot_id } => {
            let result: WorldRepairResultDto = client
                .post_json("/v1/worlds/repair", &WorldRepairRequestDto { slot_id })
                .await?;
            WorldMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            }
        }
    };
    Ok(result)
}

fn mutation_result(
    success: bool,
    message: String,
    operation_id: Option<String>,
) -> Result<WorldMutationResult, CliError> {
    if success {
        Ok(WorldMutationResult {
            message,
            operation_id,
        })
    } else {
        Err(CliError::usage(message))
    }
}
