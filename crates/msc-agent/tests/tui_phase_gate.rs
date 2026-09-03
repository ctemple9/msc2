#[allow(dead_code)]
#[path = "../src/cli/mod.rs"]
mod cli;

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

use cli::tui::app::App;
use cli::tui::layout::{LayoutMode, layout_mode};
use cli::tui::overview::OverviewState;
use cli::tui::render;
use crossterm::event::KeyCode;
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, CapabilitiesResponseDto, HelpersDto, HostOsDto,
    PermissionCategoryDto, RemoteApiStatus, ServerDto, ServerTypesDto,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn server() -> ServerDto {
    ServerDto {
        id: "paper-1".to_string(),
        name: "Paper Survival".to_string(),
        directory: "/srv/paper-survival".to_string(),
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
            agent_version: "gate-test".to_string(),
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

fn overview(permissions: Vec<PermissionCategoryDto>) -> OverviewState {
    OverviewState {
        servers: vec![server()],
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
        ..OverviewState::default()
    }
}

#[test]
fn gate_preserves_tauri_reading_order_and_terminal_structure() {
    let permissions = vec![
        PermissionCategoryDto::Players,
        PermissionCategoryDto::Worlds,
        PermissionCategoryDto::Addons,
        PermissionCategoryDto::Settings,
        PermissionCategoryDto::Admin,
    ];
    let (wide, _) = render_at(140, 42, overview(permissions), |_| {});
    for marker in [
        "Host: test-host",
        "Server: Paper Survival",
        "State: RUNNING",
        "SERVER CONTROLS",
        "SERVER IDENTITY",
        "Overview",
        "Players",
        "Worlds",
        "Performance",
        "Components",
        "Settings",
        "Files",
        "Connection Information",
        "Live Stats",
        "Server Health",
        "Activity",
        "Notes (local to this host/server)",
        "CONSOLE",
    ] {
        assert!(
            wide.contains(marker),
            "wide gate output omitted {marker:?}:\n{wide}"
        );
    }
    assert_in_order(
        &wide,
        [
            "SERVER CONTROLS",
            "SERVER IDENTITY",
            "Overview",
            "Connection Information",
            "Server Health",
            "Activity",
            "CONSOLE",
        ],
    );
    assert_no_line_exceeds(&wide, 140);
}

#[test]
fn gate_keeps_medium_and_small_paths_reachable_without_overflow() {
    assert_eq!(layout_mode(Rect::new(0, 0, 140, 42)), LayoutMode::Wide);
    assert_eq!(layout_mode(Rect::new(0, 0, 100, 30)), LayoutMode::Medium);
    assert_eq!(layout_mode(Rect::new(0, 0, 70, 20)), LayoutMode::Small);

    let state = overview(vec![
        PermissionCategoryDto::Players,
        PermissionCategoryDto::Worlds,
        PermissionCategoryDto::Addons,
        PermissionCategoryDto::Settings,
        PermissionCategoryDto::Admin,
    ]);
    let (medium, _) = render_at(100, 30, state.clone(), |app| {
        app.resize(100, 30);
    });
    for marker in [
        "SERVER CONTROLS",
        "Section: Overview",
        "Rail: shown",
        "Console: shown",
        "CONSOLE",
    ] {
        assert!(
            medium.contains(marker),
            "medium gate output omitted {marker:?}:\n{medium}"
        );
    }
    assert_no_line_exceeds(&medium, 100);

    let (collapsed, _) = render_at(100, 30, state.clone(), |app| {
        app.resize(100, 30);
        app.handle_key(KeyCode::Char('r'));
        app.handle_key(KeyCode::Char('c'));
    });
    assert!(collapsed.contains("Rail: hidden"));
    assert!(collapsed.contains("Console: hidden"));
    assert!(!collapsed.contains("› CONSOLE"));
    assert_no_line_exceeds(&collapsed, 100);

    let (small, _) = render_at(70, 20, state, |app| {
        app.resize(70, 20);
    });
    for marker in [
        "FOCUSED VIEW",
        "Host: test-host",
        "[s] sections",
        "[c] console",
        "[?] help",
    ] {
        assert!(
            small.contains(marker),
            "small gate output omitted {marker:?}:\n{small}"
        );
    }
    assert_no_line_exceeds(&small, 70);

    let (console, _) = render_at(70, 20, overview(Vec::new()), |app| {
        app.resize(70, 20);
        app.handle_key(KeyCode::Char('c'));
    });
    assert!(console.contains("› CONSOLE"));
    assert!(console.contains("No console data loaded."));
    assert_no_line_exceeds(&console, 70);

    let (help, _) = render_at(70, 20, overview(Vec::new()), |app| {
        app.resize(70, 20);
        app.handle_key(KeyCode::Char('?'));
    });
    assert!(help.contains("› KEYBOARD HELP"));
    assert!(help.contains("Raw console input stays literal Minecraft text"));
    assert_no_line_exceeds(&help, 70);
}

#[test]
fn gate_filters_sections_by_advertised_permissions() {
    let restricted = App::with_overview("host-a:48001", overview(Vec::new()));
    assert_eq!(restricted.available_tabs(), vec![0, 3]);

    let full = App::with_overview(
        "host-a:48001",
        overview(vec![
            PermissionCategoryDto::Players,
            PermissionCategoryDto::Worlds,
            PermissionCategoryDto::Addons,
            PermissionCategoryDto::Settings,
            PermissionCategoryDto::Admin,
        ]),
    );
    assert_eq!(full.available_tabs(), vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn global_navigation_remains_reachable_from_players_content() {
    let permissions = vec![
        PermissionCategoryDto::Players,
        PermissionCategoryDto::Worlds,
        PermissionCategoryDto::Addons,
        PermissionCategoryDto::Settings,
        PermissionCategoryDto::Admin,
    ];

    let mut app = App::with_overview("test-host", overview(permissions.clone()));
    app.handle_key(KeyCode::Char('2'));
    assert_eq!(app.active_tab(), 1);
    assert!(app.handle_key(KeyCode::Char('q')));

    let mut app = App::with_overview("test-host", overview(permissions.clone()));
    app.handle_key(KeyCode::Char('2'));
    let focus_before_tab = app.focus();
    assert!(!app.handle_key(KeyCode::Tab));
    assert_ne!(app.focus(), focus_before_tab);

    let mut app = App::with_overview("test-host", overview(permissions));
    app.handle_key(KeyCode::Char('2'));
    app.handle_key(KeyCode::Char('/'));
    assert!(!app.handle_key(KeyCode::Char('q')));
}

fn render_at(
    width: u16,
    height: u16,
    overview: OverviewState,
    configure: impl FnOnce(&mut App),
) -> (String, App) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_overview("test-host", overview);
    configure(&mut app);
    terminal
        .draw(|frame| render::render(frame, &mut app))
        .unwrap();
    (buffer_text(terminal.backend().buffer()), app)
}

fn assert_in_order<const N: usize>(text: &str, markers: [&str; N]) {
    let mut previous = 0;
    for marker in markers {
        let position = text
            .find(marker)
            .unwrap_or_else(|| panic!("missing ordered marker {marker:?}:\n{text}"));
        assert!(
            position >= previous,
            "marker {marker:?} appeared before the previous reading-order marker"
        );
        previous = position;
    }
}

fn assert_no_line_exceeds(text: &str, width: u16) {
    for (line, value) in text.lines().enumerate() {
        assert!(
            value.chars().count() <= width as usize,
            "line {line} exceeds {width} columns: {value:?}"
        );
    }
}

fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
