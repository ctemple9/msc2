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
use cli::tui::overview::{ConnectionVisibility, OverviewState};
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, CapabilitiesResponseDto, ConnectivityPortDiagnosticDto,
    ConnectivityPortDiagnosticsDto, ConnectivityResponseDto, HelpersDto, HostOsDto,
    PermissionCategoryDto, RemoteApiStatus, ServerDto, ServerTypesDto,
};

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

fn connectivity(join_address: Option<&str>, method: &str) -> ConnectivityResponseDto {
    let diagnostic = ConnectivityPortDiagnosticDto {
        outcome: "open".to_string(),
        detail: None,
        help_id: None,
    };
    ConnectivityResponseDto {
        server_type: "java".to_string(),
        server_name: "Paper".to_string(),
        server_running: true,
        status: "ready".to_string(),
        severity: "green".to_string(),
        headline: "Ready".to_string(),
        detail: None,
        join_address: join_address.map(str::to_string),
        method: method.to_string(),
        join_address_source: method.to_string(),
        local_listening: Some(true),
        externally_reachable: Some(method == "public"),
        port_diagnostics: ConnectivityPortDiagnosticsDto {
            local: diagnostic.clone(),
            public: diagnostic,
        },
        note: None,
        help_id: None,
    }
}

fn overview_with_permissions(permissions: Vec<PermissionCategoryDto>) -> OverviewState {
    OverviewState {
        servers: vec![server("paper-1", "Paper")],
        selected_server_id: Some("paper-1".to_string()),
        status: Some(RemoteApiStatus {
            running: true,
            active_server_id: Some("paper-1".to_string()),
            pid: Some(42),
            server_type: Some("java".to_string()),
            docker_container_running: None,
            docker_container_status: None,
            runtime: None,
        }),
        capabilities: Some(capabilities(permissions)),
        connectivity: Some(connectivity(Some("127.0.0.1:25565"), "local")),
        ..OverviewState::default()
    }
}

#[test]
fn overview_filters_tabs_by_advertised_token_permissions() {
    let state = overview_with_permissions(vec![
        PermissionCategoryDto::Players,
        PermissionCategoryDto::Settings,
    ]);

    assert_eq!(state.available_tabs(), vec![0, 1, 3, 5, 6]);
    assert_eq!(state.selected_server_name(), "Paper");
    assert_eq!(state.lifecycle_label(), "RUNNING");
}

#[test]
fn overview_distinguishes_local_public_and_hidden_connection_states() {
    let mut state = overview_with_permissions(Vec::new());
    assert_eq!(state.connection_visibility(), ConnectionVisibility::Local);

    state.connectivity = Some(connectivity(Some("play.example.test:25565"), "public"));
    assert_eq!(state.connection_visibility(), ConnectionVisibility::Public);

    state.connectivity = Some(connectivity(None, "unavailable"));
    assert_eq!(state.connection_visibility(), ConnectionVisibility::Hidden);
}

#[test]
fn notes_are_keyed_to_the_in_memory_host_and_selected_server() {
    let state = overview_with_permissions(Vec::new());
    let mut app = App::with_overview("host-a:48001", state);
    app.set_note("Watch the backup disk");
    assert_eq!(
        app.notes_for_selected_server(),
        Some("Watch the backup disk")
    );

    assert!(!app.switch_host(1));
}
