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

use cli::tui::performance::{PerformanceState, TrendMetric, trend_line};
use cli::tui::players::{
    PlayerIntent, PlayerMutation, PlayerProfile, PlayersState, profile_edition, profile_status,
    session_events_from_console,
};
use msc_api::dto::{PerformanceMetricNumberDto, PerformanceSnapshotDto};
use msc_infrastructure::console_buffer::ConsoleLine;

fn profile(id: &str, name: &str, online: bool, hidden: bool, bedrock: bool) -> PlayerProfile {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "username": name,
        "imageIdentifier": name,
        "isOnline": online,
        "isOp": false,
        "lastSeen": "2026-09-02T12:00:00Z",
        "isBedrockPlayer": bedrock,
        "isHidden": hidden,
        "stats": null
    }))
    .unwrap()
}

fn metric(value: f64) -> PerformanceMetricNumberDto {
    PerformanceMetricNumberDto {
        value,
        help_id: None,
    }
}

fn snapshot(ts: &str, tps: f64, cpu: f64, ram: f64) -> PerformanceSnapshotDto {
    PerformanceSnapshotDto {
        ts: ts.to_string(),
        tps_1m: Some(metric(tps)),
        tps_5m: Some(metric(tps - 0.1)),
        tps_15m: Some(metric(tps - 0.2)),
        players_online: Some(2),
        cpu_percent: Some(metric(cpu)),
        ram_used_mb: Some(metric(ram)),
        ram_max_mb: Some(metric(4096.0)),
        world_size_mb: Some(metric(512.0)),
        server_type: Some("java".to_string()),
        runtime: None,
    }
}

#[test]
fn players_filter_sort_and_detail_actions_keep_edition_and_history_meaning() {
    let mut state = PlayersState {
        profiles: vec![
            profile("java-profile", "Zoe", false, false, false),
            profile("bedrock-profile", "Alex", true, true, true),
        ],
        online: serde_json::from_value(serde_json::json!({
            "players": [{"name": "Alex", "uuid": "xuid-1"}], "count": 1
        }))
        .unwrap(),
        loaded: true,
        ..PlayersState::default()
    };

    assert_eq!(state.filtered_profiles().len(), 1);
    assert_eq!(
        PlayersState::display_name(state.filtered_profiles()[0]),
        "Zoe"
    );
    assert_eq!(profile_status(&state.profiles[0]), "OFFLINE");
    assert_eq!(profile_edition(&state.profiles[1]), "Bedrock");

    state.show_hidden = true;
    state.sort = cli::tui::players::ProfileSort::Name;
    state.profile_query = "alex".to_string();
    assert_eq!(
        PlayersState::display_name(state.filtered_profiles()[0]),
        "Alex"
    );
    state.detail_open = true;
    let intent = state.handle_key(crossterm::event::KeyCode::Char('d'));
    assert_eq!(
        intent,
        Some(PlayerIntent::Confirm(PlayerMutation::Delete {
            profile_id: "bedrock-profile".to_string()
        }))
    );
}

#[test]
fn bedrock_session_history_is_derived_from_the_existing_console_tail() {
    let lines = vec![
        ConsoleLine {
            ts: "2026-09-02T12:00:00Z".to_string(),
            source: "server".to_string(),
            level: None,
            text: "Alex joined the game".to_string(),
        },
        ConsoleLine {
            ts: "2026-09-02T12:02:00Z".to_string(),
            source: "server".to_string(),
            level: None,
            text: "Alex left the game".to_string(),
        },
    ];
    let events = session_events_from_console(&lines);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "left");
    assert_eq!(events[1].player_name, "Alex");
}

#[test]
fn performance_keeps_three_tps_windows_metrics_and_a_bounded_trend() {
    let mut state = PerformanceState::default();
    state.server_type = "java".to_string();
    state.record(snapshot("one", 19.5, 40.0, 1024.0), Some(false));
    state.record(snapshot("two", 20.0, 45.0, 1536.0), Some(true));
    assert_eq!(
        state.current().unwrap().tps_5m.as_ref().unwrap().value,
        19.9
    );
    assert_eq!(
        state.current().unwrap().tps_15m.as_ref().unwrap().value,
        19.8
    );
    assert_eq!(state.current().unwrap().players_online, Some(2));
    assert!(state.trend(TrendMetric::Tps).chars().count() == 2);
    assert_eq!(state.status_label(), "ONLINE");
    assert!(state.uptime_label().ends_with('s'));
    assert!(trend_line(&[]).contains("no samples"));
}
