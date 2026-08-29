//! `GET`/`POST /v1/settings` — the typed `server.properties` schema for
//! the active Java server, put behind the frozen DTO contract.
//!
//! Ports `ServerSettingsSchema.javaSections`/`.applyJava`
//! (`RemoteAPIServer+Settings.swift`) and their route wiring
//! (`settingsProvider`/`updateSettingsProvider` in
//! `AppViewModel+APIWiringSettings.swift`). `apply_java` itself is
//! P1's pure validator (`msc_domain::settings_schema`); this module is
//! the section/field DTO builder that source file's own comment says
//! is "UI/API wiring, not a domain rule" plus the disk read/write MSC 1
//! does through `ServerPropertiesManager`. Bedrock (`bedrockSections`/
//! `applyBedrock`) stays unported until Phase 10 — no active Bedrock
//! lifecycle exists yet to read a `server.properties` file for.
//!
//! `helpId` values are assigned exactly where MSC 1's baseline
//! `SettingFieldDTO.help` carried inline text (`helpid-contract.md` §4)
//! and nowhere else; the `GET /v1/help/{helpId}` route they point at is
//! still homeless (not named in any phase yet) and isn't built here.

use std::collections::HashMap;
use std::path::Path;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    PermissionCategoryDto, SettingFieldDto, SettingOptionDto, SettingRejectionDto,
    SettingsResponseDto, SettingsSectionDto, SettingsUpdateRequestDto, SettingsUpdateResultDto,
};
use msc_domain::identity::ServerType;
use msc_domain::properties::ServerPropertiesModel;
use msc_domain::settings_schema;
use msc_domain::world_profile::SettingOwner;
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};

use crate::auth::AuthenticatedCredential;
use crate::routes::bedrock::runtime_for;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

const PROPERTIES_FILE_NAME: &str = "server.properties";
const PROPERTIES_HEADER: &str = "# Modified via MSC 2\n";

pub async fn get_settings(State(state): State<LifecycleRoutesState>) -> Json<SettingsResponseDto> {
    Json(build_settings_response(&state, &StdFileSystem))
}

pub async fn update_settings(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<SettingsUpdateRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }

    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if body.changes.is_empty() {
        return invalid_body("no_changes", "changes must include at least one key.");
    }

    apply_settings_update(&state, &StdFileSystem, &body.changes)
}

fn active_server_directory(state: &LifecycleRoutesState) -> Option<(String, String)> {
    let active_id = state.active_server_id()?;
    state
        .servers()
        .into_iter()
        .find(|server| server.id == active_id)
        .map(|server| (server.name, server.directory))
}

fn build_settings_response(
    state: &LifecycleRoutesState,
    fs: &dyn FileSystem,
) -> SettingsResponseDto {
    let Some((server_name, directory)) = active_server_directory(state) else {
        return SettingsResponseDto {
            server_type: "java".to_string(),
            server_name: String::new(),
            server_running: false,
            editable: false,
            sections: Vec::new(),
            note: Some("no_active_server".to_string()),
            runtime: runtime_for(state),
        };
    };

    let Some(server) = state.active_config_server() else {
        return SettingsResponseDto {
            server_type: "java".to_string(),
            server_name,
            server_running: false,
            editable: false,
            sections: Vec::new(),
            note: Some("no_active_server".to_string()),
            runtime: runtime_for(state),
        };
    };
    if server.server_type == msc_domain::identity::ServerType::Bedrock {
        let settings = msc_application::bedrock_settings::load(fs, Path::new(&directory));
        return SettingsResponseDto {
            server_type: "bedrock".to_string(),
            server_name,
            server_running: state.status_snapshot().running,
            editable: true,
            sections: bedrock_sections(&settings.model),
            note: None,
            runtime: runtime_for(state),
        };
    }
    let model = load_properties_model(fs, Path::new(&directory));
    SettingsResponseDto {
        server_type: "java".to_string(),
        server_name,
        server_running: state.status_snapshot().running,
        editable: true,
        sections: java_sections(&model),
        note: None,
        runtime: None,
    }
}

fn apply_settings_update(
    state: &LifecycleRoutesState,
    fs: &dyn FileSystem,
    changes: &HashMap<String, String>,
) -> Response {
    let Some((_, directory)) = active_server_directory(state) else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "No server is currently active.",
        );
    };
    let dir = Path::new(&directory);
    let server_type = state
        .active_config_server()
        .map(|server| server.server_type)
        .unwrap_or(ServerType::Java);
    if server_type == ServerType::Bedrock {
        let (server_changes, mut ownership_rejections) =
            filter_server_changes(server_type, changes);
        let changes = server_changes
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let restart_required = state.status_snapshot().running
            && changes.keys().any(|key| {
                settings_schema::server_setting_contract(server_type, key).is_some_and(|contract| {
                    contract.apply_policy
                        == msc_domain::world_profile::SettingApplyPolicy::RestartRequired
                })
            });
        return match msc_application::bedrock_settings::update(fs, dir, &changes) {
            Ok(result) if result.applied_keys.is_empty() => {
                ownership_rejections.extend(result.rejected.into_iter().map(|rejection| {
                    SettingRejectionDto {
                        key: rejection.key,
                        reason: rejection.reason,
                    }
                }));
                Json(SettingsUpdateResultDto {
                    success: false,
                    message: "no_valid_changes".to_owned(),
                    restart_required,
                    applied_keys: Vec::new(),
                    rejected: (!ownership_rejections.is_empty()).then_some(ownership_rejections),
                    sections: Some(bedrock_sections(&result.settings.model)),
                    runtime: runtime_for(state),
                })
                .into_response()
            }
            Ok(result) => {
                ownership_rejections.extend(result.rejected.into_iter().map(|rejection| {
                    SettingRejectionDto {
                        key: rejection.key,
                        reason: rejection.reason,
                    }
                }));
                Json(SettingsUpdateResultDto {
                    success: true,
                    message: if ownership_rejections.is_empty() {
                        "saved".to_owned()
                    } else {
                        "saved_with_rejections".to_owned()
                    },
                    restart_required,
                    applied_keys: result.applied_keys,
                    rejected: (!ownership_rejections.is_empty()).then_some(ownership_rejections),
                    sections: Some(bedrock_sections(&result.settings.model)),
                    runtime: runtime_for(state),
                })
                .into_response()
            }
            Err(error) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            ),
        };
    }
    let (server_changes, mut ownership_rejections) = filter_server_changes(server_type, changes);
    let restart_required = state.status_snapshot().running
        && server_changes.keys().any(|key| {
            settings_schema::server_setting_contract(server_type, key).is_some_and(|contract| {
                contract.apply_policy
                    == msc_domain::world_profile::SettingApplyPolicy::RestartRequired
            })
        });

    let mut model = load_properties_model(fs, dir);
    let result = settings_schema::apply_java(&server_changes, &mut model);
    let rejected_from_schema = to_rejection_dtos(&result.rejected).unwrap_or_default();
    ownership_rejections.extend(rejected_from_schema);

    if result.applied.is_empty() {
        let sections = java_sections(&load_properties_model(fs, dir));
        return Json(SettingsUpdateResultDto {
            success: false,
            message: "no_valid_changes".to_string(),
            restart_required,
            applied_keys: Vec::new(),
            rejected: (!ownership_rejections.is_empty()).then_some(ownership_rejections),
            sections: Some(sections),
            runtime: None,
        })
        .into_response();
    }

    match save_properties_model(fs, dir, &model) {
        Ok(()) => {
            let sections = java_sections(&load_properties_model(fs, dir));
            let message = if ownership_rejections.is_empty() {
                "saved"
            } else {
                "saved_with_rejections"
            };
            Json(SettingsUpdateResultDto {
                success: true,
                message: message.to_string(),
                restart_required,
                applied_keys: result.applied,
                rejected: (!ownership_rejections.is_empty()).then_some(ownership_rejections),
                sections: Some(sections),
                runtime: None,
            })
            .into_response()
        }
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

fn filter_server_changes(
    server_type: ServerType,
    changes: &HashMap<String, String>,
) -> (HashMap<String, String>, Vec<SettingRejectionDto>) {
    let mut accepted = HashMap::new();
    let mut rejected = Vec::new();
    for (key, value) in changes {
        let contract = match server_type {
            ServerType::Java => settings_schema::java_setting_contract(key),
            ServerType::Bedrock => settings_schema::bedrock_setting_contract(key),
        };
        if contract.is_some_and(|contract| contract.owner == SettingOwner::WorldProfile) {
            rejected.push(SettingRejectionDto {
                key: key.clone(),
                reason: "world_profile_key".to_string(),
            });
        } else {
            accepted.insert(key.clone(), value.clone());
        }
    }
    (accepted, rejected)
}

fn to_rejection_dtos(rejected: &[settings_schema::Rejection]) -> Option<Vec<SettingRejectionDto>> {
    if rejected.is_empty() {
        None
    } else {
        Some(
            rejected
                .iter()
                .map(|rejection| SettingRejectionDto {
                    key: rejection.key.clone(),
                    reason: rejection.reason.clone(),
                })
                .collect(),
        )
    }
}

fn load_properties_model(fs: &dyn FileSystem, server_dir: &Path) -> ServerPropertiesModel {
    let raw = read_properties(fs, &server_dir.join(PROPERTIES_FILE_NAME));
    ServerPropertiesModel::from_dict(&raw, None)
}

fn save_properties_model(
    fs: &dyn FileSystem,
    server_dir: &Path,
    model: &ServerPropertiesModel,
) -> Result<(), msc_infrastructure::atomic_write::AtomicWriteError> {
    let path = server_dir.join(PROPERTIES_FILE_NAME);
    let existing = read_properties(fs, &path);
    let merged = model.merged_server_into(&existing);
    atomic_write(fs, &path, encode_properties(&merged).as_bytes())
}

/// Same key/value line format `ServerPropertiesManager.readProperties`
/// reads: `key=value`, blank lines and `#` comments skipped.
fn read_properties(fs: &dyn FileSystem, path: &Path) -> HashMap<String, String> {
    let Ok(bytes) = fs.read(path) else {
        return HashMap::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut properties = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        properties.insert(key.trim().to_string(), value.trim().to_string());
    }
    properties
}

/// Same shape `ServerPropertiesManager.writeProperties` writes (a leading
/// comment, then one `key=value` line per entry) except keys are sorted
/// here for deterministic output — Swift dictionary iteration order is
/// unspecified and no fixture depends on a particular on-disk ordering.
fn encode_properties(properties: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = properties.keys().collect();
    keys.sort();
    let mut out = String::from(PROPERTIES_HEADER);
    for key in keys {
        out.push_str(key);
        out.push('=');
        out.push_str(&properties[key]);
        out.push('\n');
    }
    out
}

fn java_sections(model: &ServerPropertiesModel) -> Vec<SettingsSectionDto> {
    let op_level_options = vec![
        SettingOptionDto {
            value: "1".to_string(),
            label: "1 — Bypass spawn protection".to_string(),
        },
        SettingOptionDto {
            value: "2".to_string(),
            label: "2 — Commands & command blocks".to_string(),
        },
        SettingOptionDto {
            value: "3".to_string(),
            label: "3 — Manage players".to_string(),
        },
        SettingOptionDto {
            value: "4".to_string(),
            label: "4 — All permissions".to_string(),
        },
    ];

    let runtime = SettingsSectionDto {
        id: "runtime".to_string(),
        title: "Runtime".to_string(),
        icon: "globe".to_string(),
        fields: vec![
            bool_field("pvp", "PvP", model.pvp, None),
            bool_field(
                "spawn-monsters",
                "Spawn Monsters",
                model.spawn_monsters,
                None,
            ),
            bool_field("spawn-animals", "Spawn Animals", model.spawn_animals, None),
            bool_field("spawn-npcs", "Spawn NPCs", model.spawn_npcs, None),
            bool_field("allow-nether", "Allow Nether", model.allow_nether, None),
            bool_field("allow-flight", "Allow Flight", model.allow_flight, None),
            bool_field(
                "force-gamemode",
                "Force Gamemode (server-wide)",
                model.force_gamemode,
                Some("concept.settings"),
            ),
            int_field(
                "spawn-protection",
                "Spawn Protection",
                model.spawn_protection,
                0,
                10_000,
                Some("blocks"),
                Some("settings.spawn-protection"),
            ),
        ],
    };

    let server = SettingsSectionDto {
        id: "server".to_string(),
        title: "Server".to_string(),
        icon: "slider.horizontal.3".to_string(),
        fields: vec![
            string_field(
                "motd",
                "MOTD",
                &model.motd,
                Some(200),
                Some("settings.motd"),
            ),
            int_field(
                "max-players",
                "Max Players",
                model.max_players,
                1,
                1000,
                None,
                None,
            ),
            bool_field(
                "online-mode",
                "Online Mode",
                model.online_mode,
                Some("settings.online-mode"),
            ),
            int_field(
                "view-distance",
                "View Distance",
                model.view_distance,
                3,
                32,
                Some("chunks"),
                None,
            ),
            int_field(
                "simulation-distance",
                "Simulation Distance",
                model.simulation_distance,
                3,
                32,
                Some("chunks"),
                None,
            ),
            bool_field("white-list", "Whitelist", model.whitelist, None),
            bool_field(
                "enforce-whitelist",
                "Enforce Whitelist",
                model.enforce_whitelist,
                None,
            ),
            int_field(
                "player-idle-timeout",
                "Idle Timeout",
                model.player_idle_timeout,
                0,
                1440,
                Some("min"),
                Some("settings.player-idle-timeout"),
            ),
            enum_field(
                "op-permission-level",
                "Op Permission Level",
                &model.op_permission_level.to_string(),
                op_level_options,
                None,
            ),
        ],
    };

    let network = SettingsSectionDto {
        id: "network".to_string(),
        title: "Network".to_string(),
        icon: "network".to_string(),
        fields: vec![int_field(
            "server-port",
            "Server Port (TCP)",
            model.server_port,
            1,
            65_535,
            None,
            Some("settings.server-port"),
        )],
    };

    vec![runtime, server, network]
}

fn bedrock_sections(
    model: &msc_domain::bedrock::BedrockPropertiesModel,
) -> Vec<SettingsSectionDto> {
    let field = |key: &str, label: &str, value: String, r#type: &str| SettingFieldDto {
        key: key.to_owned(),
        label: label.to_owned(),
        r#type: r#type.to_owned(),
        value,
        min_int: None,
        max_int: None,
        unit: None,
        max_length: None,
        options: None,
        help_id: None,
    };
    vec![
        SettingsSectionDto {
            id: "bedrock".to_owned(),
            title: "Bedrock".to_owned(),
            icon: "cube".to_owned(),
            fields: vec![
                field(
                    "max-players",
                    "Max Players",
                    model.max_players.to_string(),
                    "int",
                ),
                field(
                    "online-mode",
                    "Online Mode",
                    model.online_mode.to_string(),
                    "bool",
                ),
            ],
        },
        SettingsSectionDto {
            id: "network".to_owned(),
            title: "Network".to_owned(),
            icon: "network".to_owned(),
            fields: vec![
                field(
                    "server-port",
                    "Server Port (UDP)",
                    model.server_port.to_string(),
                    "int",
                ),
                field(
                    "server-portv6",
                    "Server Port (IPv6 UDP)",
                    model.server_port_v6.to_string(),
                    "int",
                ),
            ],
        },
    ]
}

fn bool_field(key: &str, label: &str, value: bool, help_id: Option<&str>) -> SettingFieldDto {
    SettingFieldDto {
        key: key.to_string(),
        label: label.to_string(),
        r#type: "bool".to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        min_int: None,
        max_int: None,
        unit: None,
        max_length: None,
        options: None,
        help_id: help_id.map(str::to_string),
    }
}

#[allow(clippy::too_many_arguments)]
fn int_field(
    key: &str,
    label: &str,
    value: i64,
    min: i64,
    max: i64,
    unit: Option<&str>,
    help_id: Option<&str>,
) -> SettingFieldDto {
    SettingFieldDto {
        key: key.to_string(),
        label: label.to_string(),
        r#type: "int".to_string(),
        value: value.to_string(),
        min_int: Some(min),
        max_int: Some(max),
        unit: unit.map(str::to_string),
        max_length: None,
        options: None,
        help_id: help_id.map(str::to_string),
    }
}

fn string_field(
    key: &str,
    label: &str,
    value: &str,
    max_length: Option<i64>,
    help_id: Option<&str>,
) -> SettingFieldDto {
    SettingFieldDto {
        key: key.to_string(),
        label: label.to_string(),
        r#type: "string".to_string(),
        value: value.to_string(),
        min_int: None,
        max_int: None,
        unit: None,
        max_length,
        options: None,
        help_id: help_id.map(str::to_string),
    }
}

fn enum_field(
    key: &str,
    label: &str,
    value: &str,
    options: Vec<SettingOptionDto>,
    help_id: Option<&str>,
) -> SettingFieldDto {
    SettingFieldDto {
        key: key.to_string(),
        label: label.to_string(),
        r#type: "enum".to_string(),
        value: value.to_string(),
        min_int: None,
        max_int: None,
        unit: None,
        max_length: None,
        options: Some(options),
        help_id: help_id.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use msc_application::import::ImportedPaperServer;
    use msc_application::lifecycle::ServerId;
    use std::collections::HashMap;

    fn imported_server(server_dir: std::path::PathBuf) -> ImportedPaperServer {
        ImportedPaperServer {
            id: ServerId::new("paper-1"),
            display_name: "Settings Route Paper".to_string(),
            paper_jar_path: server_dir.join("paper.jar"),
            server_dir,
            eula_accepted: Some(true),
            game_port: 25565,
            max_players: 20,
            world_name: "world".to_string(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        }
    }

    fn temp_server_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("msc2-settings-route-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn state_with_active_server(server_dir: std::path::PathBuf) -> LifecycleRoutesState {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server = imported_server(server_dir.clone());
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();
        state.register_imported_paper(server).unwrap();
        state.select_active_server("paper-1".to_string()).unwrap();
        state
    }

    fn settings_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Settings],
        }
    }

    #[tokio::test]
    async fn settings_route_get_reports_no_active_server_when_none_selected() {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );

        let response = build_settings_response(&state, &StdFileSystem);

        assert!(!response.editable);
        assert!(response.sections.is_empty());
        assert_eq!(response.note.as_deref(), Some("no_active_server"));
    }

    #[tokio::test]
    async fn settings_route_get_builds_java_sections_from_disk() {
        let server_dir = temp_server_dir("get");
        std::fs::write(
            server_dir.join("server.properties"),
            "difficulty=hard\nmax-players=42\nmotd=Hello\n",
        )
        .unwrap();
        let state = state_with_active_server(server_dir.clone());

        let response = build_settings_response(&state, &StdFileSystem);

        assert!(response.editable);
        assert_eq!(response.server_name, "Settings Route Paper");
        let runtime = response
            .sections
            .iter()
            .find(|section| section.id == "runtime")
            .unwrap();
        assert!(runtime.fields.iter().all(|field| {
            !matches!(
                field.key.as_str(),
                "difficulty" | "gamemode" | "level-type" | "hardcore"
            )
        }));
        let spawn_protection = runtime
            .fields
            .iter()
            .find(|f| f.key == "spawn-protection")
            .unwrap();
        assert_eq!(
            spawn_protection.help_id.as_deref(),
            Some("settings.spawn-protection")
        );
        assert!(
            runtime
                .fields
                .iter()
                .find(|f| f.key == "hardcore")
                .is_none()
        );

        let server = response
            .sections
            .iter()
            .find(|section| section.id == "server")
            .unwrap();
        let max_players = server
            .fields
            .iter()
            .find(|f| f.key == "max-players")
            .unwrap();
        assert_eq!(max_players.value, "42");
        assert_eq!(max_players.min_int, Some(1));
        assert_eq!(max_players.max_int, Some(1000));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn settings_route_post_applies_and_persists_changes() {
        let server_dir = temp_server_dir("post-apply");
        std::fs::write(
            server_dir.join("server.properties"),
            "difficulty=easy\nmax-players=20\n",
        )
        .unwrap();
        let state = state_with_active_server(server_dir.clone());

        let mut changes = HashMap::new();
        changes.insert("max-players".to_string(), "77".to_string());
        changes.insert("difficulty".to_string(), "hard".to_string());

        let response = apply_settings_update(&state, &StdFileSystem, &changes);
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SettingsUpdateResultDto = serde_json::from_slice(&body).unwrap();
        assert!(result.success);
        assert_eq!(result.message, "saved");
        assert_eq!(result.applied_keys, vec!["max-players"]);
        assert_eq!(result.rejected.unwrap()[0].key, "difficulty");

        let persisted = read_properties(&StdFileSystem, &server_dir.join(PROPERTIES_FILE_NAME));
        assert_eq!(persisted.get("max-players").map(String::as_str), Some("77"));
        assert_eq!(
            persisted.get("difficulty").map(String::as_str),
            Some("easy")
        );

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn settings_route_post_rejects_invalid_change_without_writing() {
        let server_dir = temp_server_dir("post-reject");
        std::fs::write(server_dir.join("server.properties"), "max-players=20\n").unwrap();
        let state = state_with_active_server(server_dir.clone());

        let mut changes = HashMap::new();
        changes.insert("max-players".to_string(), "not-a-number".to_string());

        let response = apply_settings_update(&state, &StdFileSystem, &changes);
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SettingsUpdateResultDto = serde_json::from_slice(&body).unwrap();
        assert!(!result.success);
        assert_eq!(result.message, "no_valid_changes");
        assert!(result.applied_keys.is_empty());
        assert_eq!(result.rejected.unwrap()[0].key, "max-players");

        let contents = std::fs::read_to_string(server_dir.join("server.properties")).unwrap();
        assert_eq!(contents, "max-players=20\n");

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn settings_route_post_reports_conflict_without_active_server() {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let mut changes = HashMap::new();
        changes.insert("max-players".to_string(), "50".to_string());

        let response = apply_settings_update(&state, &StdFileSystem, &changes);
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn settings_route_permission_check_requires_settings_category() {
        let credential = settings_credential();
        assert!(require_permission(&credential, PermissionCategoryDto::Settings).is_none());

        let other = AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::ServerControl],
        };
        assert!(require_permission(&other, PermissionCategoryDto::Settings).is_some());
    }
}
