//! P9.13 route-facing contract checks.  The service-level behavior is covered
//! by the application tests; these checks pin the agent-facing DTOs that the
//! HTTP facade and CLI share.

use msc_api::dto::{
    BroadcastSimpleResultDto, PlayitActionResultDto, ResourcePackActivateRequestDto,
    ResourcePackMutationResultDto, ResourcePacksResponseDto,
};

#[test]
fn helper_actions_carry_operation_ids_without_secret_fields() {
    let playit = serde_json::to_value(PlayitActionResultDto {
        result: "started".into(),
        message: Some("accepted".into()),
        operation_id: Some("op-playit".into()),
    })
    .unwrap();
    assert_eq!(playit["operationId"], "op-playit");
    assert!(playit.get("secret").is_none());

    let broadcast = serde_json::to_value(BroadcastSimpleResultDto {
        result: "broadcast_start_requested".into(),
        operation_id: Some("op-broadcast".into()),
    })
    .unwrap();
    assert_eq!(broadcast["operationId"], "op-broadcast");
    assert!(broadcast.get("password").is_none());
}

#[test]
fn resource_pack_route_shapes_are_machine_readable() {
    let request: ResourcePackActivateRequestDto =
        serde_json::from_value(serde_json::json!({"packId":"welcome.zip","require":true})).unwrap();
    assert_eq!(request.pack_id.as_deref(), Some("welcome.zip"));
    assert_eq!(request.require, Some(true));

    let response = ResourcePackMutationResultDto {
        success: true,
        message: "activated".into(),
        updated: Some(ResourcePacksResponseDto {
            server_type: "java".into(),
            is_java: true,
            packs: Vec::new(),
            geyser_packs: Vec::new(),
            is_geyser_available: false,
            active_pack_url: Some("http://example.test/welcome.zip".into()),
            require_pack: true,
            note: None,
        }),
    };
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(
        json["updated"]["activePackUrl"],
        "http://example.test/welcome.zip"
    );
    assert!(json.to_string().contains("activated"));
}
