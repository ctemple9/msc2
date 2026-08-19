//! Round-trips one representative instance of every P7.23 provisioning/
//! fleet/template DTO through `serde_json` and checks the result against
//! `docs/msc2/api-contract/openapi.json`'s schema for it — the same
//! `assert_conforms` depth-check `dto_conformance.rs`/
//! `world_backup_conformance.rs` already established, duplicated here per
//! those two files' own precedent rather than shared across a test-only
//! boundary.
//!
//! Test functions are prefixed `provisioning_conformance_` so the plan's
//! Verify command (a plain nextest substring filter) selects all of them.

use msc_api::dto::{
    ServerCreateRequestDto, ServerCreateResultDto, ServerDeleteRequestDto, ServerDeleteResultDto,
    ServerEulaRequestDto, ServerEulaResultDto, ServerRenameRequestDto, ServerRenameResultDto,
    TemplateItemDto, TemplateMutationRequestDto, TemplateMutationResultDto, TemplatesResponseDto,
};
use serde::Serialize;
use serde_json::Value;
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
        Some("integer") => assert!(
            instance.is_i64() || instance.is_u64(),
            "{path}: expected integer, got {instance}"
        ),
        Some("number") => assert!(
            instance.is_number(),
            "{path}: expected number, got {instance}"
        ),
        Some("boolean") => assert!(
            instance.is_boolean(),
            "{path}: expected boolean, got {instance}"
        ),
        Some(other) => panic!("{path}: unhandled schema type '{other}'"),
        None => {}
    }
}

fn check<T: Serialize>(schema_name: &str, example: &T) {
    let contract = load_contract();
    let schema = schema_for(&contract, schema_name);
    let instance = serde_json::to_value(example).unwrap();
    assert_conforms(&contract, schema, &instance, schema_name);
}

#[test]
fn provisioning_conformance_server_create_request() {
    check(
        "ServerCreateRequestDTO",
        &ServerCreateRequestDto {
            name: "Survival Realm".to_string(),
            server_type: Some("java".to_string()),
            java_flavor: Some("paper".to_string()),
            port: Some(25565),
            max_players: Some(20),
            enable_cross_play: Some(false),
            cross_play_bedrock_port: None,
            enable_playit: Some(false),
            enable_xbox_broadcast: Some(false),
            difficulty: Some("normal".to_string()),
            gamemode: Some("survival".to_string()),
            world_name: Some("World 1".to_string()),
            world_seed: None,
            version_id: Some("__latest__".to_string()),
            minecraft_version: None,
            loader_version: None,
            accept_eula: Some(true),
            bedrock_version: None,
            docker_image: None,
            java_path: None,
        },
    );
}

#[test]
fn provisioning_conformance_server_create_result() {
    check(
        "ServerCreateResultDTO",
        &ServerCreateResultDto {
            success: true,
            message: "Server creation started.".to_string(),
            operation_id: Some("op-1".to_string()),
            server_id: None,
            server_name: Some("Survival Realm".to_string()),
            warnings: Some(vec!["example warning".to_string()]),
        },
    );
}

#[test]
fn provisioning_conformance_server_delete() {
    check(
        "ServerDeleteRequestDTO",
        &ServerDeleteRequestDto {
            server_id: "srv-1".to_string(),
        },
    );
    check(
        "ServerDeleteResultDTO",
        &ServerDeleteResultDto {
            success: true,
            message: "Deleted server \"Survival Realm\".".to_string(),
            server_id: Some("srv-1".to_string()),
        },
    );
}

#[test]
fn provisioning_conformance_server_rename() {
    check(
        "ServerRenameRequestDTO",
        &ServerRenameRequestDto {
            server_id: "srv-1".to_string(),
            name: "New Name".to_string(),
        },
    );
    check(
        "ServerRenameResultDTO",
        &ServerRenameResultDto {
            success: true,
            message: "Server renamed.".to_string(),
            server_id: Some("srv-1".to_string()),
            name: Some("New Name".to_string()),
        },
    );
}

#[test]
fn provisioning_conformance_server_eula() {
    check(
        "ServerEULARequestDTO",
        &ServerEulaRequestDto {
            server_id: Some("srv-1".to_string()),
        },
    );
    check(
        "ServerEULAResultDTO",
        &ServerEulaResultDto {
            success: true,
            message: "EULA accepted.".to_string(),
            server_id: Some("srv-1".to_string()),
            accepted: Some(true),
        },
    );
}

fn sample_template_item() -> TemplateItemDto {
    TemplateItemDto {
        id: "paper:paper-1.21.4-build100.jar".to_string(),
        kind: "paper".to_string(),
        filename: "paper-1.21.4-build100.jar".to_string(),
        display_name: "Paper 1.21.4 (build 100)".to_string(),
        size_bytes: Some(45_000_000),
        modified_at: Some("2026-08-01T00:00:00Z".to_string()),
        version: Some("1.21.4".to_string()),
        build: Some(100),
    }
}

#[test]
fn provisioning_conformance_templates_response() {
    check(
        "TemplatesResponseDTO",
        &TemplatesResponseDto {
            server_name: Some("Survival Realm".to_string()),
            server_running: false,
            paper_templates: vec![sample_template_item()],
            plugin_templates: Vec::new(),
            note: None,
        },
    );
}

#[test]
fn provisioning_conformance_template_mutation_request() {
    check(
        "TemplateMutationRequestDTO",
        &TemplateMutationRequestDto {
            action: "createServer".to_string(),
            server_id: None,
            name: Some("New Server".to_string()),
            template_id: Some("paper:paper-1.21.4-build100.jar".to_string()),
            port: Some(25566),
            enable_cross_play: Some(false),
            cross_play_bedrock_port: None,
            enable_playit: Some(false),
            difficulty: Some("normal".to_string()),
            gamemode: Some("survival".to_string()),
            world_name: Some("World 1".to_string()),
            world_seed: None,
            accept_eula: Some(true),
            include_plugins: Some(true),
        },
    );
}

#[test]
fn provisioning_conformance_template_mutation_result() {
    check(
        "TemplateMutationResultDTO",
        &TemplateMutationResultDto {
            success: true,
            message: "Created server \"New Server\" from template.".to_string(),
            created_server_id: Some("srv-2".to_string()),
            created_server_name: Some("New Server".to_string()),
            exported_count: None,
            templates: Some(TemplatesResponseDto {
                server_name: Some("New Server".to_string()),
                server_running: false,
                paper_templates: vec![sample_template_item()],
                plugin_templates: Vec::new(),
                note: None,
            }),
        },
    );
}
