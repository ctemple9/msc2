//! Round-trips one representative instance of every Phase 6 world/backup
//! DTO (`P6.20`) through `serde_json` and checks the result against
//! `docs/msc2/api-contract/openapi.json`'s schema for it — the same
//! `assert_conforms` depth-check `dto_conformance.rs` already established
//! for the Phase 2/4 baseline DTOs, reused here rather than duplicated
//! with a second copy-pasted implementation.
//!
//! Test functions are prefixed `world_backup_conformance_` so the plan's
//! Verify command (a plain nextest substring filter) selects all of them.

use msc_api::dto::{
    BackupConfigResponseDto, BackupConfigUpdateRequestDto, BackupConfigUpdateResultDto,
    BackupDeleteRequestDto, BackupItemDto, BackupNowResultDto, BackupRestoreRequestDto,
    BackupRestoreResultDto, BackupsResponseDto, StagedUploadBeginRequestDto,
    StagedUploadBeginResultDto, StagedUploadCompleteResultDto, StagedUploadPurposeDto,
    WorldActivateRequestDto, WorldActivateResultDto, WorldConvertRequestDto, WorldConvertResultDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldImportRequestDto, WorldMutationResultDto,
    WorldRenameActiveWorldRequestDto, WorldRenameRequestDto, WorldRepairRequestDto,
    WorldReplaceRequestDto, WorldSlotDto, WorldSlotsResponseDto,
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

/// Same depth-check `dto_conformance.rs::assert_conforms` implements —
/// duplicated rather than shared across a test-only boundary (neither
/// test file exposes a `#[path]`-included helper module today, and
/// duplicating ~50 lines of recursive schema walking is cheaper than
/// inventing a first shared test-support crate for one function).
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
        None => {} // enum-only/untyped schema -- the enum check above already ran
    }
}

fn check<T: Serialize>(schema_name: &str, example: &T) {
    let contract = load_contract();
    let schema = schema_for(&contract, schema_name);
    let instance = serde_json::to_value(example).unwrap();
    assert_conforms(&contract, schema, &instance, schema_name);
}

fn sample_slot(active: bool) -> WorldSlotDto {
    WorldSlotDto {
        id: "11111111-1111-1111-1111-111111111111".to_string(),
        name: "Survival".to_string(),
        is_active: active,
        created_at: "2026-08-15T12:00:00Z".to_string(),
        zip_size_bytes: Some(123_456),
        world_seed: Some("42".to_string()),
        has_thumbnail: true,
    }
}

#[test]
fn world_backup_conformance_world_slots_response_matches_schema() {
    check(
        "WorldSlotsResponseDTO",
        &WorldSlotsResponseDto {
            slots: vec![sample_slot(true), sample_slot(false)],
            active_slot_id: Some("11111111-1111-1111-1111-111111111111".to_string()),
            server_running: false,
            is_repairing: Some(false),
        },
    );
}

#[test]
fn world_backup_conformance_world_slots_response_empty_matches_schema() {
    check(
        "WorldSlotsResponseDTO",
        &WorldSlotsResponseDto {
            slots: Vec::new(),
            active_slot_id: None,
            server_running: false,
            is_repairing: None,
        },
    );
}

#[test]
fn world_backup_conformance_world_create_request_matches_schema() {
    check(
        "WorldCreateRequestDTO",
        &WorldCreateRequestDto {
            name: "New World".to_string(),
            seed: Some("1234".to_string()),
        },
    );
}

#[test]
fn world_backup_conformance_world_rename_request_matches_schema() {
    check(
        "WorldRenameRequestDTO",
        &WorldRenameRequestDto {
            slot_id: "slot-1".to_string(),
            name: "Renamed".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_replace_request_matches_schema() {
    check(
        "WorldReplaceRequestDTO",
        &WorldReplaceRequestDto {
            slot_id: "slot-1".to_string(),
            source_slot_id: "slot-2".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_repair_request_matches_schema() {
    check(
        "WorldRepairRequestDTO",
        &WorldRepairRequestDto {
            slot_id: "slot-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_activate_request_matches_schema() {
    check(
        "WorldActivateRequestDTO",
        &WorldActivateRequestDto {
            slot_id: "slot-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_activate_result_matches_schema() {
    check(
        "WorldActivateResultDTO",
        &WorldActivateResultDto {
            result: "activation_started".to_string(),
            operation_id: Some("01J8XG7K9QZR3F5T6M2N8P0VBC".to_string()),
        },
    );
}

#[test]
fn world_backup_conformance_world_mutation_result_matches_schema() {
    check(
        "WorldMutationResultDTO",
        &WorldMutationResultDto {
            success: true,
            message: "saved".to_string(),
            updated: Some(WorldSlotsResponseDto {
                slots: vec![sample_slot(true)],
                active_slot_id: Some("11111111-1111-1111-1111-111111111111".to_string()),
                server_running: false,
                is_repairing: None,
            }),
        },
    );
}

#[test]
fn world_backup_conformance_world_delete_request_matches_schema() {
    check(
        "WorldDeleteRequestDTO",
        &WorldDeleteRequestDto {
            slot_id: "slot-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_duplicate_request_matches_schema() {
    check(
        "WorldDuplicateRequestDTO",
        &WorldDuplicateRequestDto {
            slot_id: "slot-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_import_request_matches_schema() {
    check(
        "WorldImportRequestDTO",
        &WorldImportRequestDto {
            name: "Imported".to_string(),
            staged_upload_id: "upload-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_export_request_matches_schema() {
    check(
        "WorldExportRequestDTO",
        &WorldExportRequestDto {
            slot_id: "slot-1".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_export_result_matches_schema() {
    check(
        "WorldExportResultDTO",
        &WorldExportResultDto {
            staged_download_id: "download-1".to_string(),
            expires_at: "2026-08-15T12:30:00Z".to_string(),
            size_bytes: 42,
        },
    );
}

#[test]
fn world_backup_conformance_world_rename_active_world_request_matches_schema() {
    check(
        "WorldRenameActiveWorldRequestDTO",
        &WorldRenameActiveWorldRequestDto {
            name: "new-world-name".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_world_convert_request_matches_schema() {
    check(
        "WorldConvertRequestDTO",
        &WorldConvertRequestDto {
            source_slot_id: "slot-1".to_string(),
            target_server_id: "server-2".to_string(),
            target_format: "JAVA_1_21_4".to_string(),
            target_name: Some("Converted".to_string()),
            target_slot_id: None,
        },
    );
}

#[test]
fn world_backup_conformance_world_convert_result_matches_schema() {
    check(
        "WorldConvertResultDTO",
        &WorldConvertResultDto {
            result: "conversion_started".to_string(),
            operation_id: "01J8XG7K9QZR3F5T6M2N8P0VBE".to_string(),
        },
    );
}

fn sample_backup_item() -> BackupItemDto {
    BackupItemDto {
        id: "world-manual-20260815-120000.zip".to_string(),
        display_name: "world (manual) - Aug 15, 2026 12:00 PM".to_string(),
        file_size: Some(654_321),
        modification_date: Some("2026-08-15T12:00:00Z".to_string()),
        is_automatic: false,
        slot_id: Some("11111111-1111-1111-1111-111111111111".to_string()),
        slot_name: Some("Survival".to_string()),
        trigger_reason: "manual".to_string(),
    }
}

#[test]
fn world_backup_conformance_backups_response_matches_schema() {
    check(
        "BackupsResponseDTO",
        &BackupsResponseDto {
            backups: vec![sample_backup_item()],
        },
    );
}

#[test]
fn world_backup_conformance_backups_response_empty_matches_schema() {
    check(
        "BackupsResponseDTO",
        &BackupsResponseDto {
            backups: Vec::new(),
        },
    );
}

#[test]
fn world_backup_conformance_backup_config_response_matches_schema() {
    check(
        "BackupConfigResponseDTO",
        &BackupConfigResponseDto {
            server_name: "Survival".to_string(),
            auto_backup_enabled: true,
            auto_backup_interval_minutes: 30,
            auto_backup_max_count: 10,
            interval_options: vec![15, 30, 60, 120],
            note: None,
        },
    );
}

#[test]
fn world_backup_conformance_backup_config_update_request_matches_schema() {
    check(
        "BackupConfigUpdateRequestDTO",
        &BackupConfigUpdateRequestDto {
            auto_backup_enabled: Some(true),
            auto_backup_interval_minutes: Some(60),
            auto_backup_max_count: None,
        },
    );
}

#[test]
fn world_backup_conformance_backup_config_update_result_matches_schema() {
    check(
        "BackupConfigUpdateResultDTO",
        &BackupConfigUpdateResultDto {
            success: true,
            message: "saved".to_string(),
            config: Some(BackupConfigResponseDto {
                server_name: "Survival".to_string(),
                auto_backup_enabled: true,
                auto_backup_interval_minutes: 60,
                auto_backup_max_count: 10,
                interval_options: vec![15, 30, 60, 120],
                note: None,
            }),
        },
    );
}

#[test]
fn world_backup_conformance_backup_now_result_matches_schema() {
    check(
        "BackupNowResultDTO",
        &BackupNowResultDto {
            result: "backup_started".to_string(),
            operation_id: Some("01J8XG7K9QZR3F5T6M2N8P0VBF".to_string()),
        },
    );
}

#[test]
fn world_backup_conformance_backup_restore_request_matches_schema() {
    check(
        "BackupRestoreRequestDTO",
        &BackupRestoreRequestDto {
            backup_id: "world-manual-20260815-120000.zip".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_backup_restore_result_matches_schema() {
    check(
        "BackupRestoreResultDTO",
        &BackupRestoreResultDto {
            result: "restore_started".to_string(),
            operation_id: Some("01J8XG7K9QZR3F5T6M2N8P0VBG".to_string()),
        },
    );
}

#[test]
fn world_backup_conformance_backup_delete_request_matches_schema() {
    check(
        "BackupDeleteRequestDTO",
        &BackupDeleteRequestDto {
            backup_id: "world-manual-20260815-120000.zip".to_string(),
        },
    );
}

#[test]
fn world_backup_conformance_staged_upload_begin_request_matches_schema() {
    check(
        "StagedUploadBeginRequestDTO",
        &StagedUploadBeginRequestDto {
            purpose: StagedUploadPurposeDto::WorldImport,
            content_type: Some("application/zip".to_string()),
        },
    );
}

#[test]
fn world_backup_conformance_staged_upload_begin_result_matches_schema() {
    check(
        "StagedUploadBeginResultDTO",
        &StagedUploadBeginResultDto {
            staged_upload_id: "upload-1".to_string(),
            upload_path: "/v1/staged-uploads/upload-1".to_string(),
            expires_at: "2026-08-15T12:30:00Z".to_string(),
            max_bytes: 10_737_418_240,
        },
    );
}

#[test]
fn world_backup_conformance_staged_upload_complete_result_matches_schema() {
    check(
        "StagedUploadCompleteResultDTO",
        &StagedUploadCompleteResultDto {
            staged_upload_id: "upload-1".to_string(),
            received_bytes: 4096,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        },
    );
}
