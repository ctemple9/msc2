//! Validates a representative instance of every schema `docs/msc2/addons/
//! phase8-api.md` (P8.9) adds or changes in `docs/msc2/api-contract/
//! openapi.json` -- the same `assert_conforms` depth-check `dto_conformance.rs`/
//! `world_backup_conformance.rs`/`provisioning_conformance.rs` already
//! established, duplicated here per those three files' own precedent rather
//! than shared across a test-only boundary.
//!
//! Unlike those three files, this one checks raw `serde_json::Value`
//! literals rather than serialized Rust structs: P8.9 is a contract-freeze
//! step (Files: `phase8-api.md`, `openapi.json`, `client-capability-matrix.csv`,
//! this file) that runs before any Phase 8 Rust DTO exists in `msc-api` --
//! those are P8.10-P8.26's job. Once P8.24 wires real routes and gives these
//! schemas real Rust types, the shared `AddonsResponseDTO`/`ComponentUpdateRequestDTO`/
//! etc. examples already covered by `dto_conformance.rs` grow their new
//! P8.9 fields there too; this file is not meant to be extended forever in
//! parallel with a typed version -- it exists to prove the schemas
//! themselves are self-consistent and instantiable before anything is built
//! against them.
//!
//! Test functions are prefixed `phase8_conformance_` so the plan's Verify
//! command (a plain nextest substring filter) selects all of them.

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
                    } else {
                        panic!("{path}: field '{key}' is not declared on this schema");
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

fn check_value(schema_name: &str, instance: Value) {
    let contract = load_contract();
    let schema = schema_for(&contract, schema_name);
    assert_conforms(&contract, schema, &instance, schema_name);
}

// --- New schemas: staged-upload-backed modpack workflow (SS3) ---------------

#[test]
fn phase8_conformance_modpack_inspection_result_mrpack() {
    check_value(
        "ModpackInspectionResultDTO",
        json!({
            "success": true,
            "message": "Fabulously Optimized 13.3.0 inspected.",
            "format": "mrpack",
            "packName": "Fabulously Optimized",
            "packVersion": "13.3.0",
            "minecraftVersion": "1.21.1",
            "loaderName": "fabric",
            "loaderVersion": "0.16.9",
            "fileCount": 214,
            "clientOnlyFileCount": 12,
            "manualFiles": [],
            "warnings": []
        }),
    );
}

#[test]
fn phase8_conformance_modpack_inspection_result_curseforge_with_manual_files() {
    check_value(
        "ModpackInspectionResultDTO",
        json!({
            "success": true,
            "message": "Fabulously Optimized 13.3.0 (CurseForge) inspected.",
            "format": "curseforge",
            "packName": "Fabulously Optimized",
            "packVersion": "13.3.0",
            "minecraftVersion": "1.21.1",
            "loaderName": "fabric",
            "loaderVersion": "0.16.9",
            "fileCount": 214,
            "clientOnlyFileCount": 12,
            "manualFiles": [
                {
                    "fileId": "8287121",
                    "fileName": "entityculling-fabric-1.7.3+minecraft.1.21.1.jar",
                    "projectName": "Entity Culling"
                }
            ],
            "warnings": ["1 file requires manual completion before this pack can finish importing."]
        }),
    );
}

#[test]
fn phase8_conformance_modpack_import_request_and_result() {
    check_value(
        "ModpackImportRequestDTO",
        json!({
            "stagedUploadId": "su-1",
            "action": "import"
        }),
    );
    check_value(
        "ModpackImportRequestDTO",
        json!({
            "stagedUploadId": "su-2",
            "action": "replace"
        }),
    );
    check_value(
        "ModpackImportResultDTO",
        json!({
            "success": true,
            "message": "Import started.",
            "operationId": "op-1",
            "pendingManualFiles": []
        }),
    );
    check_value(
        "ModpackImportResultDTO",
        json!({
            "success": true,
            "message": "Import paused: 1 file needs manual completion.",
            "operationId": "op-2",
            "pendingManualFiles": [
                {
                    "fileId": "8287121",
                    "fileName": "entityculling-fabric-1.7.3+minecraft.1.21.1.jar",
                    "projectName": "Entity Culling"
                }
            ]
        }),
    );
}

#[test]
fn phase8_conformance_modpack_manual_file_request_and_result() {
    check_value(
        "ModpackManualFileRequestDTO",
        json!({
            "fileId": "8287121",
            "stagedUploadId": "su-3"
        }),
    );
    check_value(
        "ModpackManualFileResultDTO",
        json!({
            "success": true,
            "message": "File accepted; import resumed.",
            "operationId": "op-2",
            "remainingManualFiles": [],
            "allFilesResolved": true
        }),
    );
}

// --- Additive fields on existing schemas (SS2) ------------------------------

#[test]
fn phase8_conformance_staged_upload_begin_request_new_purposes() {
    for purpose in ["modpack-archive", "addon-local-file"] {
        check_value(
            "StagedUploadBeginRequestDTO",
            json!({"purpose": purpose, "contentType": "application/zip"}),
        );
    }
    check_value(
        "StagedUploadBeginRequestDTO",
        json!({
            "purpose": "curseforge-manual-file",
            "contentType": "application/java-archive",
            "operationId": "op-2",
            "fileId": "8287121"
        }),
    );
}

#[test]
fn phase8_conformance_catalog_install_request_local_file_shape() {
    check_value(
        "CatalogInstallRequestDTO",
        json!({"stagedUploadId": "su-4"}),
    );
    check_value(
        "CatalogInstallRequestDTO",
        json!({
            "projectId": "P7dR8mSH",
            "slug": "iris",
            "title": "Iris"
        }),
    );
}

#[test]
fn phase8_conformance_catalog_install_result_with_operation_and_dependencies() {
    check_value(
        "CatalogInstallResultDTO",
        json!({
            "success": true,
            "message": "Install started.",
            "projectId": "P7dR8mSH",
            "operationId": "op-3",
            "installedDependencies": ["sodium-fabric-0.6.13+mc1.21.1"]
        }),
    );
}

#[test]
fn phase8_conformance_component_update_request_toggle_link_and_source_shapes() {
    check_value(
        "ComponentUpdateRequestDTO",
        json!({"jarStem": "iris-fabric-1.8.4", "enabled": false}),
    );
    check_value(
        "ComponentUpdateRequestDTO",
        json!({"jarStem": "worldedit-7.3.0", "linkProjectId": "1u6JkXh5"}),
    );
    check_value(
        "ComponentUpdateRequestDTO",
        json!({
            "jarStem": "EssentialsX-2.22.0",
            "sourceUrl": "https://github.com/EssentialsX/Essentials/releases"
        }),
    );
    check_value(
        "ComponentUpdateRequestDTO",
        json!({"jarStem": "EssentialsX-2.22.0", "removeSource": true}),
    );
}

#[test]
fn phase8_conformance_addon_update_result_with_operation_id() {
    check_value(
        "AddonUpdateResultDTO",
        json!({"result": "updated", "jarStem": "iris-fabric-1.8.4", "count": 1, "operationId": "op-4"}),
    );
    check_value(
        "AddonUpdateResultDTO",
        json!({"result": "enabled", "jarStem": "iris-fabric-1.8.4", "count": 1}),
    );
}

#[test]
fn phase8_conformance_addons_response_with_provider_note() {
    check_value(
        "AddonsResponseDTO",
        json!({
            "addons": [],
            "isResolving": false,
            "serverSupportsAddons": true,
            "packManaged": false,
            "note": "Modrinth was unreachable during this resolve pass; showing last-known state."
        }),
    );
}

#[test]
fn phase8_conformance_component_status_with_not_yet_implemented_note() {
    check_value(
        "ComponentStatusDTO",
        json!({
            "name": "geyser",
            "isUpToDate": false,
            "updatable": false,
            "note": "Geyser/Floodgate update checks stay Phase 9."
        }),
    );
}

#[test]
fn phase8_conformance_client_export_response_staged_download_shape() {
    check_value(
        "ClientExportResponseDTO",
        json!({
            "serverName": "Fabulously Optimized",
            "serverType": "java",
            "exportKind": "zip",
            "isPaperLike": false,
            "items": [],
            "selectedCount": 0,
            "zipFileName": "Fabulously Optimized-client-1.21.1.zip",
            "stagedDownloadId": "sd-1"
        }),
    );
}

#[test]
fn phase8_conformance_server_create_request_staged_pack_shape() {
    check_value(
        "ServerCreateRequestDTO",
        json!({
            "name": "Modded Realm",
            "serverType": "java",
            "javaFlavor": "fabric",
            "stagedModpackUploadId": "su-5"
        }),
    );
}

#[test]
fn phase8_conformance_health_repair_result_with_operation_id() {
    check_value(
        "HealthRepairResultDTO",
        json!({
            "success": true,
            "message": "Repair started.",
            "operationId": "op-5"
        }),
    );
}
