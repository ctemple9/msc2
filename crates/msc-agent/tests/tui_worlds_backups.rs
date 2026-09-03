mod test_cli {
    pub use crate::cli::{CliError, CommonArgs};

    pub fn resolve_base_url(common: &CommonArgs) -> String {
        common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", common.host, common.port))
    }

    pub fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
        common
            .token
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/mod.rs"]
mod cli;

use cli::tui::backups::{BackupIntent, BackupMutation, BackupsState};
use cli::tui::worlds::{WorldIntent, WorldMutation, WorldsState};
use crossterm::event::KeyCode;
use msc_api::dto::{
    BackupConfigResponseDto, BackupItemDto, BackupsResponseDto, WorldSlotDto, WorldSlotsResponseDto,
};

fn slot(id: &str, name: &str, active: bool) -> WorldSlotDto {
    WorldSlotDto {
        id: id.to_string(),
        name: name.to_string(),
        is_active: active,
        created_at: "2026-09-02T12:00:00Z".to_string(),
        zip_size_bytes: Some(1024),
        world_seed: None,
        has_thumbnail: false,
    }
}

#[test]
fn worlds_keep_active_identity_and_backup_context_on_the_selected_slot() {
    let mut worlds = WorldsState {
        response: Some(WorldSlotsResponseDto {
            slots: vec![
                slot("slot-a", "Survival", true),
                slot("slot-b", "Creative", false),
            ],
            active_slot_id: Some("slot-a".to_string()),
            server_running: false,
            is_repairing: Some(false),
        }),
        loaded: true,
        ..WorldsState::default()
    };

    assert_eq!(worlds.active_slot().unwrap().id, "slot-a");
    worlds.handle_key(KeyCode::Down);
    assert_eq!(worlds.selected_slot_name(), Some("Creative"));
    assert_eq!(worlds.handle_key(KeyCode::Enter), None);
    assert!(worlds.detail_open);

    assert_eq!(
        worlds.handle_key(KeyCode::Char('b')),
        Some(WorldIntent::OpenBackups)
    );
    assert_eq!(
        worlds.handle_key(KeyCode::Char('d')),
        Some(WorldIntent::Confirm(WorldMutation::Delete {
            slot_id: "slot-b".to_string()
        }))
    );
}

#[test]
fn world_copy_requires_a_source_and_backup_actions_name_the_selected_archive() {
    let mut worlds = WorldsState {
        response: Some(WorldSlotsResponseDto {
            slots: vec![
                slot("slot-a", "Survival", true),
                slot("slot-b", "Creative", false),
            ],
            active_slot_id: Some("slot-a".to_string()),
            server_running: false,
            is_repairing: None,
        }),
        loaded: true,
        selected_slot: 1,
        detail_open: true,
        ..WorldsState::default()
    };
    worlds.handle_key(KeyCode::Char('p'));
    for character in "slot-a".chars() {
        worlds.handle_key(KeyCode::Char(character));
    }
    assert_eq!(
        worlds.handle_key(KeyCode::Enter),
        Some(WorldIntent::Confirm(WorldMutation::Copy {
            destination_slot_id: "slot-b".to_string(),
            source_slot_id: "slot-a".to_string(),
        }))
    );

    let mut backups = BackupsState {
        response: Some(BackupsResponseDto {
            backups: vec![BackupItemDto {
                id: "backup-1".to_string(),
                display_name: "Survival before update".to_string(),
                file_size: Some(2048),
                modification_date: None,
                is_automatic: false,
                slot_id: Some("slot-b".to_string()),
                slot_name: Some("Creative".to_string()),
                trigger_reason: "manual".to_string(),
            }],
            runtime: None,
        }),
        config: Some(BackupConfigResponseDto {
            server_name: "Paper".to_string(),
            auto_backup_enabled: true,
            auto_backup_interval_minutes: 30,
            auto_backup_max_count: 5,
            interval_options: vec![15, 30],
            note: None,
            runtime: None,
        }),
        loaded: true,
        context_slot_id: Some("slot-b".to_string()),
        open: true,
        ..BackupsState::default()
    };

    assert_eq!(
        backups.handle_key(KeyCode::Char('r')),
        Some(BackupIntent::Confirm(BackupMutation::Restore {
            backup_id: "backup-1".to_string()
        }))
    );
}
