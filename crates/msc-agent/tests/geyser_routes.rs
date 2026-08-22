//! The frozen Geyser contract is intentionally configuration-only: install
//! state is visible, but no route treats Geyser/Floodgate as client add-ons.

use msc_api::dto::{GeyserConfigResponseDto, GeyserConfigUpdateRequestDto};

#[test]
fn geyser_config_wire_types_keep_listener_values_optional() {
    let response = GeyserConfigResponseDto {
        server_name: "Test server".into(),
        server_type: "java".into(),
        is_geyser_installed: false,
        address: None,
        port: None,
        config_file_exists: false,
        note: Some("geyser_not_installed".into()),
    };
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["isGeyserInstalled"], false);
    assert!(json["address"].is_null());
    let update: GeyserConfigUpdateRequestDto =
        serde_json::from_value(serde_json::json!({"port": 19132})).unwrap();
    assert_eq!(update.port, Some(19132));
    assert_eq!(update.address, None);
}
