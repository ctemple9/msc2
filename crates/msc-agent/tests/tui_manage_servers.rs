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

use cli::tui::manage_servers::{ManageIntent, ManageMutation, ManageServersState};
use cli::tui::server_editor::{EditorIntent, EditorMutation, EditorSurface, ServerEditorState};
use crossterm::event::KeyCode;
use msc_api::dto::{JavaRuntimeDto, ServerDto};

fn server(id: &str, name: &str) -> ServerDto {
    ServerDto {
        id: id.to_string(),
        name: name.to_string(),
        directory: format!("/srv/{name}"),
        server_type: "java".to_string(),
        java_flavor: Some("paper".to_string()),
        game_port: Some(25565),
        bedrock_port: None,
        first_start_required: Some(false),
        playit_enabled: Some(false),
        xbox_broadcast_enabled: Some(false),
        host_address: Some("127.0.0.1".to_string()),
        runtime: None,
    }
}

fn type_text(state: &mut ManageServersState, text: &str) {
    for character in text.chars() {
        state.handle_key(KeyCode::Char(character));
    }
    state.handle_key(KeyCode::Enter);
}

#[test]
fn create_flow_keeps_the_desktop_choices_visible_until_review() {
    let mut manage = ManageServersState::from_servers(vec![server("paper-1", "Paper")]);
    manage.open();
    manage.handle_key(KeyCode::Char('c'));
    type_text(&mut manage, "New Paper");
    type_text(&mut manage, "java");
    type_text(&mut manage, "paper");
    type_text(&mut manage, "25570");
    type_text(&mut manage, "Survival");
    type_text(&mut manage, "yes");

    assert!(manage.create_step_is_review());
    assert!(manage.input.is_none());
    assert_eq!(
        manage.handle_key(KeyCode::Enter),
        Some(ManageIntent::Confirm(ManageMutation::Create(
            cli::tui::manage_servers::CreateDraft {
                name: "New Paper".to_string(),
                server_type: Some("java".to_string()),
                java_flavor: Some("paper".to_string()),
                port: Some(25570),
                world_name: Some("Survival".to_string()),
                accept_eula: true,
            }
        )))
    );
}

#[test]
fn manage_detail_routes_delete_through_the_shared_confirmation_intent() {
    let mut manage = ManageServersState::from_servers(vec![server("paper-1", "Paper")]);
    manage.open();
    manage.handle_key(KeyCode::Enter);

    assert_eq!(
        manage.handle_key(KeyCode::Char('d')),
        Some(ManageIntent::Confirm(ManageMutation::Delete {
            server_id: "paper-1".to_string()
        }))
    );
}

#[test]
fn editor_keeps_host_path_and_java_runtime_actions_separate() {
    let mut editor = ServerEditorState {
        server: Some(server("paper-1", "Paper")),
        java_runtimes: vec![JavaRuntimeDto {
            name: "Temurin 21".to_string(),
            executable_path: "/usr/bin/java-21".to_string(),
            major_version: Some(21),
        }],
        ..ServerEditorState::default()
    };
    editor.input = Some((
        cli::tui::server_editor::EditorInputKind::Directory,
        "/remote/minecraft".to_string(),
    ));
    assert_eq!(
        editor.handle_key(KeyCode::Enter),
        Some(EditorIntent::Confirm(EditorMutation::SetDirectory {
            server_id: "paper-1".to_string(),
            directory: "/remote/minecraft".to_string(),
        },))
    );

    editor.surface = EditorSurface::Java;
    assert_eq!(
        editor.handle_key(KeyCode::Enter),
        Some(EditorIntent::Confirm(EditorMutation::SetJavaPath {
            path: "/usr/bin/java-21".to_string()
        }))
    );
}
