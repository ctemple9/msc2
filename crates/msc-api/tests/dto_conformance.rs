//! Serializes each `msc-api` DTO's example value and validates the result
//! against its matching schema in `docs/msc2/api-contract/openapi.json`
//! (P2.8) — P0.23's schema-depth discipline (nesting reaches real content,
//! not a stub), now pointed at Rust output instead of the schema's own
//! shape.
//!
//! Only covers the five schemas the skeletal agent actually serves this
//! phase: `ErrorDTO`, `OperationDTO`, `CapabilitiesDTO`, `HealthResponseDTO`
//! (with `HealthCardDTO` nested), and `RemoteAPIStatus`.
//!
//! Test functions are prefixed `dto_conformance_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

use msc_api::dto::{
    BedrockBackendDto, BedrockSupportDto, CapabilitiesDto, ErrorDto, HealthCardDto,
    HealthResponseDto, HelpersDto, HostOsDto, OperationDto, OperationProgressDto,
    OperationStateDto, PerformanceMetricNumberDto, PerformanceSnapshotDto, PermissionCategoryDto,
    RemoteApiStatus, ServerImportResultDto, ServerTypesDto,
};
use serde_json::{Value, json};
use std::path::Path;

fn load_contract() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/api-contract/openapi.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&text).expect("openapi.json is valid JSON")
}

fn schema_for<'a>(contract: &'a Value, name: &str) -> &'a Value {
    contract["components"]["schemas"]
        .get(name)
        .unwrap_or_else(|| panic!("openapi.json has no schema named '{name}'"))
}

fn resolve<'a>(contract: &'a Value, schema: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(reference) => {
            let name = reference.rsplit('/').next().expect("well-formed $ref");
            schema_for(contract, name)
        }
        None => schema,
    }
}

/// A depth-check, not a full JSON Schema validator: every required field
/// must be present, every present field's declared type/enum must match,
/// and `$ref`/`items`/`properties` are followed recursively so nested
/// content is checked too — not just the top level. Fields the instance
/// carries but the schema doesn't declare are not flagged; this checks
/// what P2.8 *does* declare, not `additionalProperties: false`.
fn assert_conforms(contract: &Value, schema: &Value, instance: &Value, path: &str) {
    let schema = resolve(contract, schema);

    if instance.is_null() {
        let nullable = schema
            .get("nullable")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(nullable, "{path}: null not allowed by schema {schema}");
        return;
    }

    if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
        assert!(
            enum_values.contains(instance),
            "{path}: {instance} not one of {enum_values:?}"
        );
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let obj = instance
                .as_object()
                .unwrap_or_else(|| panic!("{path}: expected object, got {instance}"));
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for field in required {
                    let field = field.as_str().expect("required entries are strings");
                    assert!(
                        obj.contains_key(field),
                        "{path}: missing required field '{field}'"
                    );
                }
            }
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for (key, value) in obj {
                    if let Some(prop_schema) = properties.get(key) {
                        assert_conforms(contract, prop_schema, value, &format!("{path}.{key}"));
                    }
                }
            }
        }
        Some("array") => {
            let arr = instance
                .as_array()
                .unwrap_or_else(|| panic!("{path}: expected array, got {instance}"));
            if let Some(items) = schema.get("items") {
                for (i, item) in arr.iter().enumerate() {
                    assert_conforms(contract, items, item, &format!("{path}[{i}]"));
                }
            }
        }
        Some("string") => assert!(
            instance.is_string(),
            "{path}: expected string, got {instance}"
        ),
        Some("integer") => {
            assert!(
                instance.is_i64() || instance.is_u64(),
                "{path}: expected integer, got {instance}"
            )
        }
        Some("number") => assert!(
            instance.is_number(),
            "{path}: expected number, got {instance}"
        ),
        Some("boolean") => assert!(
            instance.is_boolean(),
            "{path}: expected boolean, got {instance}"
        ),
        Some(other) => panic!("{path}: unhandled schema type '{other}'"),
        None => {} // enum-only or untyped schema — the enum check above already ran
    }
}

/// P2.4 §5's own worked example.
#[test]
fn dto_conformance_error_dto_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "ErrorDTO");
    let example = ErrorDto {
        code: "not_found".to_string(),
        message: "No server named 'survival2' exists.".to_string(),
        help_id: Some("servers.not-found".to_string()),
        details: Some(json!({ "requestedName": "survival2" })),
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "ErrorDTO");
}

/// `operation-model.md` §2's own worked example (a `running` operation
/// with progress and no result/error yet).
#[test]
fn dto_conformance_operation_dto_running_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "OperationDTO");
    let example = OperationDto {
        id: "01J8XG7K9QZR3F5T6M2N8P0VBC".to_string(),
        r#type: "demo-install".to_string(),
        target: Some("survival2".to_string()),
        state: OperationStateDto::Running,
        progress: Some(OperationProgressDto {
            current: 42,
            total: 100,
        }),
        status_line: Some("Downloading Java 21 runtime (42/86 MB)".to_string()),
        result: None,
        error: None,
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "OperationDTO");
}

/// A `failed` operation, exercising the nested `ErrorDTO` `$ref` inside
/// `OperationDTO.error` that the running-example above leaves at `null`.
#[test]
fn dto_conformance_operation_dto_failed_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "OperationDTO");
    let example = OperationDto {
        id: "01J8XG7K9QZR3F5T6M2N8P0VBD".to_string(),
        r#type: "java-download".to_string(),
        target: None,
        state: OperationStateDto::Failed,
        progress: None,
        status_line: Some("Download failed".to_string()),
        result: None,
        error: Some(ErrorDto {
            code: "internal_error".to_string(),
            message: "Connection reset while downloading Java 21.".to_string(),
            help_id: None,
            details: None,
        }),
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "OperationDTO");
}

/// P6.44's atomic cooperative-cancellation wire rule is part of the contract,
/// not an agent-only implementation detail: acceptance is always pending,
/// while every terminal-first outcome is a conflict.
#[test]
fn dto_conformance_operation_cancellation_declares_atomic_pending_response() {
    let contract = load_contract();
    let responses = &contract["paths"]["/v1/operations/{id}/cancel"]["post"]["responses"];
    assert!(responses.get("200").is_none());
    assert!(responses.get("202").is_some());
    assert!(responses.get("404").is_some());
    assert!(responses.get("409").is_some());
    assert_eq!(
        responses["202"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/OperationDTO"
    );
}

#[test]
fn dto_conformance_server_import_is_a_durable_accepted_operation() {
    let contract = load_contract();
    let responses = &contract["paths"]["/v1/servers/import"]["post"]["responses"];
    assert_eq!(
        responses["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ServerImportScanResponseDTO"
    );
    assert_eq!(
        responses["202"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ServerImportResultDTO"
    );

    let accepted = ServerImportResultDto {
        success: true,
        message: "Server import accepted.".to_string(),
        operation_id: Some("op-123-1".to_string()),
        server_id: None,
        server_name: None,
        imported: None,
        skipped: None,
        replaced: None,
    };
    assert_conforms(
        &contract,
        schema_for(&contract, "ServerImportResultDTO"),
        &serde_json::to_value(accepted).unwrap(),
        "ServerImportResultDTO",
    );
}

/// `capability-model.md` §3's own worked example.
#[test]
fn dto_conformance_capabilities_dto_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "CapabilitiesDTO");
    let example = CapabilitiesDto {
        agent_version: "2.0.0-dev".to_string(),
        api_major: 1,
        api_minor: 0,
        host_os: HostOsDto::Macos,
        permissions: vec![
            PermissionCategoryDto::ServerControl,
            PermissionCategoryDto::Players,
            PermissionCategoryDto::Settings,
        ],
        server_types: ServerTypesDto {
            vanilla: true,
            paper: true,
            fabric: true,
            forge: true,
            neoforge: true,
            bedrock: BedrockSupportDto {
                supported: false,
                backend: None,
            },
        },
        helpers: HelpersDto {
            playit: false,
            duckdns: false,
            geyser: false,
        },
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "CapabilitiesDTO");
}

/// A macOS host with the VZ-sidecar Bedrock backend, exercising the
/// non-`null` `backend` enum branch the first example leaves untouched.
#[test]
fn dto_conformance_capabilities_dto_with_bedrock_backend_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "CapabilitiesDTO");
    let example = CapabilitiesDto {
        agent_version: "2.0.0-dev".to_string(),
        api_major: 1,
        api_minor: 0,
        host_os: HostOsDto::Macos,
        permissions: vec![PermissionCategoryDto::Admin],
        server_types: ServerTypesDto {
            vanilla: true,
            paper: true,
            fabric: true,
            forge: true,
            neoforge: true,
            bedrock: BedrockSupportDto {
                supported: true,
                backend: Some(BedrockBackendDto::VzSidecar),
            },
        },
        helpers: HelpersDto {
            playit: true,
            duckdns: false,
            geyser: true,
        },
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "CapabilitiesDTO");
}

/// A native process server, all optional fields populated.
#[test]
fn dto_conformance_status_running_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "RemoteAPIStatus");
    let example = RemoteApiStatus {
        running: true,
        active_server_id: Some("survival2".to_string()),
        pid: Some(51234),
        server_type: Some("paper".to_string()),
        docker_container_running: None,
        docker_container_status: None,
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "RemoteAPIStatus");
}

/// Only `running` is required — a stopped server with every optional
/// field absent (serialized as `null`) must still conform.
#[test]
fn dto_conformance_status_stopped_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "RemoteAPIStatus");
    let example = RemoteApiStatus {
        running: false,
        active_server_id: None,
        pid: None,
        server_type: None,
        docker_container_running: None,
        docker_container_status: None,
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "RemoteAPIStatus");
}

#[test]
fn dto_conformance_performance_snapshot_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "PerformanceSnapshotDTO");
    let example = PerformanceSnapshotDto {
        ts: "2026-08-02T00:00:00Z".to_string(),
        tps_1m: Some(PerformanceMetricNumberDto {
            value: 19.8,
            help_id: Some("performance.tps".to_string()),
        }),
        players_online: Some(3),
        cpu_percent: Some(PerformanceMetricNumberDto {
            value: 42.0,
            help_id: Some("performance.cpu".to_string()),
        }),
        ram_used_mb: Some(PerformanceMetricNumberDto {
            value: 768.0,
            help_id: Some("performance.ram".to_string()),
        }),
        ram_max_mb: Some(PerformanceMetricNumberDto {
            value: 2048.0,
            help_id: Some("performance.ram".to_string()),
        }),
        world_size_mb: Some(PerformanceMetricNumberDto {
            value: 512.0,
            help_id: Some("performance.world-size".to_string()),
        }),
        server_type: Some("paper".to_string()),
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "PerformanceSnapshotDTO");
}

/// A health response with two cards, one carrying every optional field
/// and one carrying only the required ones — exercises `HealthCardDTO`'s
/// nested `$ref` inside `HealthResponseDTO.cards`.
#[test]
fn dto_conformance_health_response_matches_schema() {
    let contract = load_contract();
    let schema = schema_for(&contract, "HealthResponseDTO");
    let example = HealthResponseDto {
        server_type: "paper".to_string(),
        server_name: "survival2".to_string(),
        server_running: true,
        overall_severity: "warning".to_string(),
        cards: vec![
            HealthCardDto {
                id: "tick-lag".to_string(),
                title: "Tick lag detected".to_string(),
                short_label: "Lag".to_string(),
                severity: "warning".to_string(),
                detail: Some("TPS has been below 15 for 5 minutes.".to_string()),
                icon_system_name: "gauge".to_string(),
                action_label: Some("View performance".to_string()),
                action_code: Some("open-performance".to_string()),
                help_id: Some("health.tick-lag".to_string()),
            },
            HealthCardDto {
                id: "disk-space".to_string(),
                title: "Disk space OK".to_string(),
                short_label: "Disk".to_string(),
                severity: "ok".to_string(),
                detail: None,
                icon_system_name: "internaldrive".to_string(),
                action_label: None,
                action_code: None,
                help_id: None,
            },
        ],
        note: None,
    };
    let instance = serde_json::to_value(&example).unwrap();
    assert_conforms(&contract, schema, &instance, "HealthResponseDTO");
}
