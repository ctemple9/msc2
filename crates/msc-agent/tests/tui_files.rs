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

use cli::tui::app::App;
use cli::tui::files::{FileItem, FilePreview, FilesIntent, FilesResponse, FilesState};
use cli::tui::overview::OverviewState;
use crossterm::event::KeyCode;
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, CapabilitiesResponseDto, HelpersDto, HostOsDto,
    PermissionCategoryDto, ServerTypesDto,
};

fn item(id: &str, name: &str, path: &str, is_directory: bool, is_previewable: bool) -> FileItem {
    FileItem {
        id: id.to_string(),
        name: name.to_string(),
        path: path.to_string(),
        is_directory,
        size_bytes: (!is_directory).then_some(128),
        modified_at: Some("2026-09-02T12:00:00Z".to_string()),
        file_extension: (!is_directory).then(|| "properties".to_string()),
        is_previewable,
    }
}

fn capabilities(permissions: Vec<PermissionCategoryDto>) -> CapabilitiesResponseDto {
    CapabilitiesResponseDto {
        base: CapabilitiesDto {
            agent_version: "test".to_string(),
            api_major: 1,
            api_minor: 0,
            host_os: HostOsDto::Macos,
            permissions,
            server_types: ServerTypesDto {
                vanilla: true,
                paper: true,
                fabric: true,
                forge: true,
                neoforge: true,
                bedrock: BedrockSupportDto {
                    supported: true,
                    backend: None,
                    runtime: None,
                },
            },
            helpers: HelpersDto {
                playit: false,
                duckdns: false,
                geyser: false,
                tailscale: Some(false),
            },
        },
        world_settings: None,
    }
}

#[test]
fn files_keep_scoped_navigation_preview_and_reported_paths() {
    let mut files = FilesState {
        response: Some(FilesResponse {
            server_name: Some("Paper".to_string()),
            path: "plugins".to_string(),
            parent_path: Some("".to_string()),
            items: vec![
                item("config", "config", "plugins/config", true, false),
                item(
                    "paper",
                    "paper.properties",
                    "plugins/paper.properties",
                    false,
                    true,
                ),
                item("jar", "plugin.jar", "plugins/plugin.jar", false, false),
            ],
            note: None,
        }),
        loaded: true,
        selected: 0,
        ..FilesState::default()
    };

    assert_eq!(files.report_path(), "Server Root / plugins");
    assert_eq!(
        files.handle_key(KeyCode::Enter),
        Some(FilesIntent::Navigate(Some("plugins/config".to_string())))
    );

    files.selected = 1;
    assert_eq!(
        files.handle_key(KeyCode::Enter),
        Some(FilesIntent::Preview("plugins/paper.properties".to_string()))
    );
    files.preview = Some(FilePreview {
        success: true,
        message: "ok".to_string(),
        path: Some("plugins/paper.properties".to_string()),
        name: Some("paper.properties".to_string()),
        size_bytes: Some(128),
        content: Some("motd=MSC".to_string()),
        encoding: Some("text".to_string()),
        truncated: Some(false),
    });
    files.detail_open = true;
    assert_eq!(
        files.handle_key(KeyCode::Char('y')),
        Some(FilesIntent::ReportPath(
            "Server Root / plugins/paper.properties".to_string()
        ))
    );
    assert_eq!(files.handle_key(KeyCode::Esc), None);
    assert!(!files.detail_open);

    files.selected = 2;
    assert_eq!(files.handle_key(KeyCode::Enter), None);
    assert_eq!(
        files.status.as_deref(),
        Some("That file type is not previewable")
    );
    assert_eq!(
        files.handle_key(KeyCode::Char('b')),
        Some(FilesIntent::Navigate(Some("".to_string())))
    );
}

#[test]
fn files_tab_is_hidden_without_admin_permission() {
    let without_admin = OverviewState {
        capabilities: Some(capabilities(vec![])),
        ..OverviewState::default()
    };
    let app = App::with_overview("host-a:48001", without_admin);
    assert!(!app.available_tabs().contains(&6));

    let with_admin = OverviewState {
        capabilities: Some(capabilities(vec![PermissionCategoryDto::Admin])),
        ..OverviewState::default()
    };
    let app = App::with_overview("host-a:48001", with_admin);
    assert!(app.available_tabs().contains(&6));
}
