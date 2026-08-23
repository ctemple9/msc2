//! P10.6 freezes the shared Bedrock API contract before runtime code exists.
//! These examples check the additive DTOs and the existing shared routes
//! without pretending that a particular host has a usable BDS installation.

use msc_api::dto::{BedrockRuntimeStateDto, HostOsDto, ServerCreateResultDto};
use serde_json::{Value, json};
use std::path::Path;

fn openapi() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/api-contract/openapi.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read openapi.json"))
        .expect("openapi.json is valid JSON")
}

fn schema<'a>(doc: &'a Value, name: &str) -> &'a Value {
    &doc["components"]["schemas"][name]
}

fn resolve<'a>(doc: &'a Value, value: &'a Value) -> &'a Value {
    value["$ref"]
        .as_str()
        .map(|reference| schema(doc, reference.rsplit('/').next().expect("schema ref name")))
        .unwrap_or(value)
}

fn assert_conforms(doc: &Value, raw_schema: &Value, value: &Value, path: &str) {
    let schema = resolve(doc, raw_schema);
    if value.is_null() {
        assert!(
            schema["nullable"].as_bool().unwrap_or(false),
            "{path}: null is not allowed"
        );
        return;
    }
    if let Some(values) = schema["enum"].as_array() {
        assert!(
            values.contains(value),
            "{path}: {value} is outside {values:?}"
        );
    }
    match schema["type"].as_str() {
        Some("object") => {
            let object = value.as_object().expect("object value");
            for required in schema["required"].as_array().into_iter().flatten() {
                let field = required.as_str().expect("required field name");
                assert!(object.contains_key(field), "{path}: missing {field}");
            }
            for (key, child) in object {
                let child_schema = schema["properties"]
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}: undeclared field {key}"));
                assert_conforms(doc, child_schema, child, &format!("{path}.{key}"));
            }
        }
        Some("array") => {
            for (index, child) in value.as_array().expect("array value").iter().enumerate() {
                assert_conforms(doc, &schema["items"], child, &format!("{path}[{index}]"));
            }
        }
        Some("string") => assert!(value.is_string(), "{path}: expected string"),
        Some("integer") => assert!(value.is_i64() || value.is_u64(), "{path}: expected integer"),
        Some("boolean") => assert!(value.is_boolean(), "{path}: expected boolean"),
        Some(other) => panic!("{path}: unhandled schema type {other}"),
        None => {}
    }
}

fn check(doc: &Value, name: &str, value: Value) {
    assert_conforms(doc, schema(doc, name), &value, name);
}

#[test]
fn phase10_conformance_runtime_state_and_capability_unavailable_error() {
    let doc = openapi();
    check(
        &doc,
        "BedrockRuntimeStateDTO",
        json!({
            "state": "unavailable",
            "backend": null,
            "hostOs": "macos",
            "reasonCode": "no_test_hardware",
            "message": "Bedrock is unavailable on this host.",
            "helpId": "bedrock.runtime-unavailable"
        }),
    );
    check(
        &doc,
        "ErrorDTO",
        json!({
            "code": "capability_unavailable",
            "message": "Bedrock is unavailable on this host.",
            "helpId": "bedrock.runtime-unavailable",
            "details": {
                "capability": "bedrock-runtime",
                "serverType": "bedrock",
                "state": "unavailable",
                "backend": null,
                "reasonCode": "no_test_hardware",
                "hostOs": "macos"
            }
        }),
    );
}

#[test]
fn phase10_conformance_rust_runtime_field_is_additive_and_error_mapping_is_shared() {
    let runtime = BedrockRuntimeStateDto {
        state: "unavailable".to_owned(),
        backend: None,
        host_os: Some(HostOsDto::Macos),
        reason_code: Some("no_test_hardware".to_owned()),
        message: Some("Bedrock is unavailable on this host.".to_owned()),
        help_id: Some("bedrock.runtime-unavailable".to_owned()),
    };
    let result = ServerCreateResultDto {
        success: false,
        message: "Bedrock is unavailable on this host.".to_owned(),
        operation_id: None,
        server_id: None,
        server_name: None,
        warnings: None,
        runtime: Some(runtime.clone()),
    };
    let encoded = serde_json::to_value(&result).expect("serialize runtime disclosure");
    assert_eq!(encoded["runtime"]["state"], "unavailable");
    assert_eq!(encoded["runtime"]["reasonCode"], "no_test_hardware");

    let old_client_shape: ServerCreateResultDto = serde_json::from_value(json!({
        "success": true,
        "message": "Java creation accepted."
    }))
    .expect("old Java response remains decodable");
    assert!(old_client_shape.runtime.is_none());

    let error = runtime.capability_unavailable_error();
    assert_eq!(error.code, "capability_unavailable");
    let details = error.details.as_ref().expect("structured details");
    assert_eq!(details["capability"], "bedrock-runtime");
    assert_eq!(details["serverType"], "bedrock");
}

#[test]
fn phase10_conformance_shared_bedrock_creation_settings_players_and_metrics() {
    let doc = openapi();
    check(
        &doc,
        "ServerCreateRequestDTO",
        json!({"name": "Bedrock Town", "serverType": "bedrock", "bedrockVersion": "1.21.80"}),
    );
    check(
        &doc,
        "ServerCreateResultDTO",
        json!({
            "success": true,
            "message": "Bedrock creation accepted.",
            "operationId": "op-bedrock-create",
            "runtime": {"state": "provisioning_required", "backend": "native", "hostOs": "linux", "reasonCode": null, "message": null, "helpId": null}
        }),
    );
    check(
        &doc,
        "SettingsResponseDTO",
        json!({
            "serverType": "bedrock", "serverName": "Bedrock Town", "serverRunning": false,
            "editable": true, "sections": [],
            "runtime": {"state": "available", "backend": "native", "hostOs": "linux", "reasonCode": null, "message": null, "helpId": null}
        }),
    );
    check(
        &doc,
        "PlayersResponseDTO",
        json!({
            "players": [{"name": "Alex", "uuid": "2535464815"}], "count": 1,
            "runtime": {"state": "available", "backend": "native", "hostOs": "linux", "reasonCode": null, "message": null, "helpId": null}
        }),
    );
    check(
        &doc,
        "PerformanceSnapshotDTO",
        json!({
            "ts": "2026-08-22T05:00:00Z", "playersOnline": 1,
            "runtime": {"state": "available", "backend": "native", "hostOs": "linux", "reasonCode": null, "message": null, "helpId": null}
        }),
    );
}

#[test]
fn phase10_conformance_bedrock_capabilities_and_operation_cancellation() {
    let doc = openapi();
    check(
        &doc,
        "CapabilitiesDTO",
        json!({
            "agentVersion": "2.0.0-dev", "apiMajor": 1, "apiMinor": 0, "hostOs": "macos",
            "permissions": ["serverControl", "players"],
            "serverTypes": {
                "vanilla": true, "paper": true, "fabric": true, "forge": true, "neoforge": true,
                "bedrock": {
                    "supported": false, "backend": null,
                    "runtime": {"state": "unavailable", "backend": null, "hostOs": "macos", "reasonCode": "no_test_hardware", "message": "Bedrock is unavailable on this host.", "helpId": "bedrock.runtime-unavailable"}
                }
            },
            "helpers": {"playit": false, "duckdns": false, "geyser": false}
        }),
    );
    check(
        &doc,
        "OperationDTO",
        json!({
            "id": "op-bedrock-start", "type": "bedrock-start", "target": "server-1",
            "state": "running", "progress": null, "statusLine": "Starting Bedrock",
            "cancelable": true, "result": null
        }),
    );
}

#[test]
fn phase10_conformance_routes_stay_shared_and_websocket_contract_is_unchanged_in_shape() {
    let doc = openapi();
    for path in [
        "/v1/servers/create",
        "/v1/start",
        "/v1/stop",
        "/v1/command",
        "/v1/settings",
        "/v1/players",
        "/v1/allowlist",
        "/v1/performance",
        "/v1/console/tail",
        "/v1/versions",
        "/v1/components/version",
        "/v1/operations/{id}/cancel",
    ] {
        assert!(
            doc["paths"].get(path).is_some(),
            "missing shared route {path}"
        );
        assert!(
            !path.starts_with("/v1/bedrock/"),
            "parallel Bedrock route added: {path}"
        );
    }
    let websocket_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/msc2/api-contract/websocket-v1.json");
    let websocket: Value = serde_json::from_str(
        &std::fs::read_to_string(websocket_path).expect("read websocket-v1.json"),
    )
    .expect("websocket-v1.json is valid JSON");
    let channels = websocket["channels"]
        .as_array()
        .expect("websocket channels");
    let console = channels
        .iter()
        .find(|channel| channel["name"] == "console")
        .expect("console channel");
    assert_eq!(console["payload"]["type"], "ConsoleLineDTO");
    let operations = channels
        .iter()
        .find(|channel| channel["name"] == "operation-progress")
        .expect("operation channel");
    assert_eq!(operations["payload"]["type"], "OperationDTO");
}
