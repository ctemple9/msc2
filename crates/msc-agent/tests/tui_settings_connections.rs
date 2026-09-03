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
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/mod.rs"]
mod cli;

use cli::tui::access::{AccessIntent, AccessMutation, AccessState, AllowlistEntry};
use cli::tui::connections::{
    ConnectionInputKind, ConnectionIntent, ConnectionMutation, ConnectionsState,
};
use cli::tui::health::{HealthIntent, HealthMutation, HealthState};
use cli::tui::settings::{SettingsIntent, SettingsMutation, SettingsState};
use crossterm::event::KeyCode;
use msc_api::dto::{
    HealthCardDto, HealthProblemsResponseDto, HealthResponseDto, SettingFieldDto,
    SettingsResponseDto, SettingsSectionDto, StartupProblemDto,
};
use std::collections::HashMap;

#[test]
fn settings_edit_uses_agent_schema_and_preserves_sparse_changes() {
    let mut settings = SettingsState {
        response: Some(SettingsResponseDto {
            server_type: "java".to_string(),
            server_name: "Paper".to_string(),
            server_running: false,
            editable: true,
            sections: vec![SettingsSectionDto {
                id: "world".to_string(),
                title: "World".to_string(),
                icon: "globe".to_string(),
                fields: vec![SettingFieldDto {
                    key: "difficulty".to_string(),
                    label: "Difficulty".to_string(),
                    r#type: "string".to_string(),
                    value: "normal".to_string(),
                    min_int: None,
                    max_int: None,
                    unit: None,
                    max_length: None,
                    options: None,
                    help_id: Some("settings.difficulty".to_string()),
                }],
            }],
            note: None,
            runtime: None,
        }),
        loaded: true,
        ..SettingsState::default()
    };
    settings.handle_key(KeyCode::Enter);
    for character in "hard".chars() {
        settings.handle_key(KeyCode::Char(character));
    }

    assert_eq!(
        settings.handle_key(KeyCode::Enter),
        Some(SettingsIntent::Confirm(SettingsMutation::Update {
            changes: HashMap::from([("difficulty".to_string(), "normalhard".to_string())]),
        }))
    );
}

#[test]
fn broadcast_credentials_are_parsed_as_a_write_only_mutation() {
    let mut connections = ConnectionsState {
        surface: cli::tui::connections::ConnectionSurface::Services,
        ..ConnectionsState::default()
    };
    connections.handle_key(KeyCode::Char('e'));
    for character in "cameron@example.test|secret-password|Cameron".chars() {
        connections.handle_key(KeyCode::Char(character));
    }

    assert_eq!(
        connections.handle_key(KeyCode::Enter),
        Some(ConnectionIntent::Confirm(
            ConnectionMutation::BroadcastCredentials {
                email: "cameron@example.test".to_string(),
                password: "secret-password".to_string(),
                gamertag: "Cameron".to_string(),
            },
        ))
    );
    assert_eq!(connections.input, None);
    assert_eq!(
        ConnectionInputKind::BroadcastCredentials.prompt(),
        "email|password|gamertag"
    );
}

#[test]
fn health_repair_requires_selecting_a_reported_action() {
    let mut health = HealthState {
        health: Some(HealthResponseDto {
            server_type: "java".to_string(),
            server_name: "Paper".to_string(),
            server_running: false,
            overall_severity: "critical".to_string(),
            cards: vec![HealthCardDto {
                id: "startup".to_string(),
                title: "Startup".to_string(),
                short_label: "Startup".to_string(),
                severity: "critical".to_string(),
                detail: None,
                icon_system_name: "xmark".to_string(),
                action_label: None,
                action_code: None,
                help_id: None,
            }],
            note: None,
        }),
        problems: Some(HealthProblemsResponseDto {
            server_type: "java".to_string(),
            server_running: false,
            is_soft_fail: false,
            problems: vec![StartupProblemDto {
                id: "missing-mod".to_string(),
                kind: "missingDependency".to_string(),
                kind_title: "Missing dependency".to_string(),
                icon_system_name: "warning".to_string(),
                offender_name: "example-mod".to_string(),
                requirement: None,
                installed_file: None,
                installed_jar_stem: None,
                missing_dependency: Some("library".to_string()),
                raw_excerpt: "library is missing".to_string(),
                is_repairing: false,
                available_actions: vec!["disable".to_string(), "install".to_string()],
                modrinth_url: None,
                help_id: None,
            }],
            note: None,
        }),
        loaded: true,
        ..HealthState::default()
    };
    health.handle_key(KeyCode::Enter);

    assert_eq!(
        health.handle_key(KeyCode::Char('2')),
        Some(HealthIntent::Confirm(HealthMutation::Repair {
            problem_id: "missing-mod".to_string(),
            action: "install".to_string(),
        }))
    );
}

#[test]
fn access_mutations_target_the_selected_allowlist_entry() {
    let mut access = AccessState {
        allowlist: vec![AllowlistEntry {
            name: "PlayerOne".to_string(),
        }],
        loaded: true,
        ..AccessState::default()
    };
    assert_eq!(
        access.handle_key(KeyCode::Char('x')),
        Some(AccessIntent::Confirm(AccessMutation::AllowlistRemove {
            name: "PlayerOne".to_string(),
        }))
    );
}
