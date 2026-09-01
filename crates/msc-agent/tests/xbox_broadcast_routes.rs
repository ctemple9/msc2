use msc_api::dto::{
    BroadcastAuthPromptDto, BroadcastCredentialsDto, BroadcastCredentialsStatusDto,
    BroadcastJarDownloadResultDto, BroadcastSimpleResultDto, BroadcastStatusDto,
    NotificationEventDto,
};
use serde_json::json;

#[test]
fn broadcast_route_dtos_match_the_frozen_wire_names_and_redact_nothing_by_accident() {
    let status = serde_json::to_value(BroadcastStatusDto {
        xbox_broadcast_running: true,
        bedrock_broadcast_running: false,
        gamertag: None,
    })
    .unwrap();
    assert_eq!(
        status,
        json!({"xboxBroadcastRunning": true, "bedrockBroadcastRunning": false})
    );

    let prompt = serde_json::to_value(BroadcastAuthPromptDto {
        is_present: true,
        code: Some("ABCD-1234".into()),
        link_url: Some("https://microsoft.com/devicelogin".into()),
    })
    .unwrap();
    assert_eq!(prompt["linkURL"], "https://microsoft.com/devicelogin");

    let credentials: BroadcastCredentialsDto = serde_json::from_value(json!({
        "email": "test@example.com", "password": "s3cr3t", "gamertag": "TestPlayer"
    }))
    .unwrap();
    assert_eq!(credentials.email, "test@example.com");

    let credential_status = serde_json::to_value(BroadcastCredentialsStatusDto {
        email: Some("test@example.com".into()),
        gamertag: Some("TestPlayer".into()),
        has_password: true,
    })
    .unwrap();
    assert_eq!(credential_status["hasPassword"], true);
    assert!(credential_status.get("password").is_none());

    let result = serde_json::to_value(BroadcastJarDownloadResultDto {
        success: true,
        message: "downloaded".into(),
        filename: Some("MCXboxBroadcastStandalone-v3.0.2.jar".into()),
        operation_id: Some("op-broadcast-1".into()),
    })
    .unwrap();
    assert_eq!(result["operationId"], "op-broadcast-1");
    assert!(result.get("password").is_none());

    let simple = serde_json::to_value(BroadcastSimpleResultDto {
        result: "broadcast_start_requested".into(),
        operation_id: None,
    })
    .unwrap();
    assert_eq!(simple["result"], "broadcast_start_requested");
}

#[test]
fn notification_route_dto_carries_all_baseline_event_kinds() {
    for kind in [
        "server_started",
        "server_stopped",
        "player_joined",
        "player_left",
        "helper_failed",
        "connectivity_changed",
    ] {
        let event = NotificationEventDto {
            id: "event-1".into(),
            server_id: "paper-1".into(),
            occurred_at_iso8601: "2026-08-22T05:00:00Z".into(),
            kind: kind.into(),
            title: "status".into(),
            body: "safe status".into(),
            help_id: None,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["kind"], kind);
        assert!(value.get("password").is_none());
    }
}
