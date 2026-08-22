//! P9.4 freezes networking/helper schemas before P9.5+ adds Rust DTOs and
//! routes. These raw JSON examples prove the contract is instantiable without
//! pretending that the future implementation already exists.

use serde_json::{Value, json};
use std::path::Path;

fn contract() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/api-contract/openapi.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read openapi.json"))
        .expect("openapi.json is valid JSON")
}

fn schema<'a>(contract: &'a Value, name: &str) -> &'a Value {
    &contract["components"]["schemas"][name]
}

fn resolve<'a>(contract: &'a Value, schema_value: &'a Value) -> &'a Value {
    if let Some(reference) = schema_value["$ref"].as_str() {
        schema(
            contract,
            reference.rsplit('/').next().expect("schema ref name"),
        )
    } else {
        schema_value
    }
}

fn assert_conforms(contract: &Value, schema: &Value, value: &Value, path: &str) {
    let schema = resolve(contract, schema);
    if value.is_null() {
        assert!(
            schema["nullable"].as_bool().unwrap_or(false),
            "{path}: null is not allowed"
        );
        return;
    }
    if let Some(choices) = schema["enum"].as_array() {
        assert!(
            choices.contains(value),
            "{path}: {value} is outside {choices:?}"
        );
    }
    match schema["type"].as_str() {
        Some("object") => {
            let object = value.as_object().expect("object value");
            for field in schema["required"].as_array().into_iter().flatten() {
                assert!(
                    object.contains_key(field.as_str().expect("field name")),
                    "{path}: missing {field}"
                );
            }
            for (key, child) in object {
                let child_schema = schema["properties"]
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}: undeclared field {key}"));
                assert_conforms(contract, child_schema, child, &format!("{path}.{key}"));
            }
        }
        Some("array") => {
            for (index, child) in value.as_array().expect("array value").iter().enumerate() {
                assert_conforms(
                    contract,
                    &schema["items"],
                    child,
                    &format!("{path}[{index}]"),
                );
            }
        }
        Some("string") => assert!(value.is_string(), "{path}: expected string"),
        Some("integer") => assert!(value.is_i64() || value.is_u64(), "{path}: expected integer"),
        Some("boolean") => assert!(value.is_boolean(), "{path}: expected boolean"),
        Some(other) => panic!("{path}: unhandled type {other}"),
        None => {}
    }
}

fn check(name: &str, value: Value) {
    let contract = contract();
    assert_conforms(&contract, schema(&contract, name), &value, name);
}

#[test]
fn phase9_conformance_connectivity_distinguishes_provider_failure() {
    check(
        "ConnectivityResponseDTO",
        json!({
            "serverType": "paper", "serverName": "Town", "serverRunning": true,
            "status": "degraded", "severity": "warning", "headline": "Public check unavailable",
            "method": "duckdns", "joinAddressSource": "duckdns",
            "portDiagnostics": {
                "local": {"outcome": "open"},
                "public": {"outcome": "unavailable", "detail": "provider timeout", "helpId": "connectivity.diagnostic-unavailable"}
            }
        }),
    );
}

#[test]
fn phase9_conformance_helper_operations_expose_no_secret() {
    check(
        "PlayitActionResultDTO",
        json!({"result": "start_accepted", "operationId": "op-playit-1"}),
    );
    check(
        "BroadcastJarDownloadResultDTO",
        json!({"success": true, "message": "Download started.", "operationId": "op-broadcast-1"}),
    );
    check(
        "BroadcastSimpleResultDTO",
        json!({"result": "stop_accepted", "operationId": "op-broadcast-2"}),
    );
}

#[test]
fn phase9_conformance_notification_event_preserves_baseline_player_event() {
    check(
        "NotificationEventDTO",
        json!({
            "id": "event-1", "serverId": "server-1", "occurredAtISO8601": "2026-08-22T05:00:00Z",
            "kind": "player_joined", "title": "Alex joined", "body": "Alex joined Town."
        }),
    );
}

#[test]
fn phase9_conformance_notification_stream_and_helper_routes_are_operation_aware() {
    let contract = contract();
    assert_eq!(
        contract["paths"]["/v1/playit/start"]["post"]["responses"]["202"]["description"],
        "Tunnel start accepted; operationId is populated"
    );
    assert!(
        contract["paths"]["/v1/broadcast/download-jar"]["post"]["responses"]
            .get("202")
            .is_some()
    );
    let channel = contract["channels"].as_array();
    assert!(
        channel.is_none(),
        "OpenAPI deliberately contains HTTP routes only"
    );
    let websocket_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/msc2/api-contract/websocket-v1.json");
    let websocket: Value = serde_json::from_str(
        &std::fs::read_to_string(websocket_path).expect("read websocket contract"),
    )
    .expect("websocket contract is valid JSON");
    let notifications = websocket["channels"]
        .as_array()
        .expect("channels")
        .iter()
        .find(|channel| channel["path"] == "/v1/notifications/stream")
        .expect("notification stream");
    assert_eq!(
        notifications["payload"]["schema"],
        "#/components/schemas/NotificationEventDTO"
    );
}
