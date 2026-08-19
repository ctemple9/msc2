//! P7.23: `GET`/`POST /v1/templates` — Paper/plugin template listing,
//! exporting the active server as a template, and creating a new server
//! from one. Backed by `msc_application::templates` over the real
//! `AppConfig.paper_template_dir`/`plugin_template_dir` and `StdFileSystem`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use msc_api::dto::{
    PermissionCategoryDto, TemplateItemDto, TemplateMutationRequestDto, TemplateMutationResultDto,
    TemplatesResponseDto,
};
use msc_application::templates::{
    self, CreateFromTemplateRequest, ServerTemplates, TemplateListItem,
};
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};
use crate::routes::servers::is_create_flow_flavor;

pub fn router(state: LifecycleRoutesState) -> Router {
    Router::new()
        .route("/templates", get(list).post(mutate))
        .with_state(state)
}

/// Duplicated from `routes/servers.rs`'s own private `agent_home_dir` —
/// the same "small pure helper, one call site per file" precedent that
/// file's own `iso8601_now`/`civil_from_days` already establish, rather
/// than promoting a one-line HOME resolver across a module boundary.
fn agent_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn iso8601_now() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    system_time_to_iso8601(duration.as_secs()).unwrap_or_default()
}

fn system_time_to_iso8601(epoch_secs: u64) -> Option<String> {
    let days = epoch_secs / 86_400;
    let remainder = epoch_secs % 86_400;
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days as i64);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Howard Hinnant's `civil_from_days`, duplicated from `routes/servers.rs`'s
/// own private copy per this crate's already-established precedent for
/// this specific algorithm (see that file's own doc comment).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn template_item_to_dto(item: TemplateListItem) -> TemplateItemDto {
    let modified_at = item
        .modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .filter(|d| d.as_secs() > 0)
        .and_then(|d| system_time_to_iso8601(d.as_secs()));
    TemplateItemDto {
        id: item.id,
        kind: item.kind.to_string(),
        filename: item.filename,
        display_name: item.display_name,
        size_bytes: Some(item.size_bytes as i64),
        modified_at,
        version: item.version,
        build: item.build,
    }
}

fn build_templates_response(
    state: &LifecycleRoutesState,
    templates: ServerTemplates,
) -> TemplatesResponseDto {
    let active = state.active_config_server();
    TemplatesResponseDto {
        server_name: active.as_ref().map(|server| server.display_name.clone()),
        server_running: state.status_snapshot().running,
        paper_templates: templates
            .paper
            .into_iter()
            .map(template_item_to_dto)
            .collect(),
        plugin_templates: templates
            .plugin
            .into_iter()
            .map(template_item_to_dto)
            .collect(),
        note: None,
    }
}

fn load_templates(state: &LifecycleRoutesState) -> Result<ServerTemplates, String> {
    let cfg = state.app_config_snapshot();
    templates::list_server_templates(
        &StdFileSystem,
        Path::new(&cfg.paper_template_dir),
        Path::new(&cfg.plugin_template_dir),
        &agent_home_dir(),
    )
    .map_err(|error| error.to_string())
}

pub async fn list(State(state): State<LifecycleRoutesState>) -> Response {
    match load_templates(&state) {
        Ok(templates) => Json(build_templates_response(&state, templates)).into_response(),
        Err(message) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &message,
        ),
    }
}

pub async fn mutate(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<TemplateMutationRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };

    match body.action.as_str() {
        "exportServer" => export_server(&state, &body),
        "createServer" => create_server_from_template(&state, &body).await,
        _ => invalid_body(
            "invalid_action",
            "action must be exportServer or createServer.",
        ),
    }
}

/// `templateMutationProvider`'s `"exportServer"` case — always targets
/// the currently active server (`export_server_as_template` has no
/// running-server refusal, per `templates.rs`'s own doc correction to
/// this step's original plan text). A `serverId` the request supplies
/// must name that same active server; an absent one is treated as "use
/// whichever server is active."
fn export_server(state: &LifecycleRoutesState, body: &TemplateMutationRequestDto) -> Response {
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::NOT_FOUND,
            "server_not_found",
            "No active server.",
        );
    };
    if let Some(requested_id) = body.server_id.as_deref().map(str::trim)
        && !requested_id.is_empty()
        && requested_id != server.id
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "server_not_found",
            "Server not found.",
        );
    }

    let cfg = state.app_config_snapshot();
    let include_plugins = body.include_plugins.unwrap_or(true);
    let exported = templates::export_server_as_template(
        &StdFileSystem,
        &agent_home_dir(),
        Path::new(&cfg.paper_template_dir),
        Path::new(&cfg.plugin_template_dir),
        Path::new(&server.server_dir),
        &server.paper_jar_path,
        server.server_type == ServerType::Java,
        include_plugins,
    );

    let templates = match load_templates(state) {
        Ok(templates) => templates,
        Err(message) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &message,
            );
        }
    };
    Json(TemplateMutationResultDto {
        success: true,
        message: format!("Exported {} item(s) as templates.", exported.exported_count),
        created_server_id: None,
        created_server_name: None,
        exported_count: Some(exported.exported_count as i64),
        templates: Some(build_templates_response(state, templates)),
    })
    .into_response()
}

/// `templateMutationProvider`'s `"createServer"` case: `templateId` is
/// `TemplateListItem.id`'s own `"<kind>:<filename>"` shape — only a
/// `"paper:"`-kind entry is a real create source (a plugin jar isn't a
/// server jar), and only a create-flow flavor within that (Pufferfish
/// and friends are excluded from the create flow everywhere else in
/// this phase, per `is_create_flow_flavor`'s own doc). Synchronous, not
/// operation-backed: `TemplateMutationResultDTO` carries no
/// `operationId` field at all in the frozen contract — unlike
/// `POST /v1/servers/create`, a template create is a local file copy,
/// not a network download.
async fn create_server_from_template(
    state: &LifecycleRoutesState,
    body: &TemplateMutationRequestDto,
) -> Response {
    let Some(template_id) = body
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return invalid_body("template_required", "templateId is required.");
    };
    let Some(("paper", filename)) = template_id.split_once(':') else {
        return error_response(
            StatusCode::NOT_FOUND,
            "template_not_found",
            "Template not found.",
        );
    };
    let Some(safe_name) =
        msc_domain::provisioning::trimmed_server_name(body.name.as_deref().unwrap_or(""))
    else {
        return invalid_body("name_required", "name is required.");
    };

    let cfg = state.app_config_snapshot();
    let template_path = Path::new(&cfg.paper_template_dir).join(filename);
    if StdFileSystem.stat(&template_path).is_err() {
        return error_response(
            StatusCode::NOT_FOUND,
            "template_not_found",
            "Template not found.",
        );
    }
    let flavor = templates::template_flavor_for_filename(filename);
    if !is_create_flow_flavor(flavor) {
        return error_response(
            StatusCode::CONFLICT,
            "unsupported_template",
            "This template's flavor is not supported for server creation.",
        );
    }

    let port: u16 = match body.port.unwrap_or(25565).try_into() {
        Ok(port) => port,
        Err(_) => return invalid_body("invalid_body", "port must be between 0 and 65535."),
    };
    let cross_play_bedrock_port = body
        .cross_play_bedrock_port
        .and_then(|port| u16::try_from(port).ok());
    let enable_cross_play = body.enable_cross_play.unwrap_or(false);
    let enable_playit = body.enable_playit.unwrap_or(false);
    let accept_eula = body.accept_eula.unwrap_or(false);
    let servers_root = state.servers_root();
    let folder_name = msc_domain::provisioning::folder_name_from_safe_name(&safe_name);
    if StdFileSystem
        .stat(&servers_root.join("java").join(&folder_name))
        .is_ok()
    {
        return error_response(
            StatusCode::CONFLICT,
            "internal_error",
            &format!("A server folder named \"{folder_name}\" already exists."),
        );
    }

    let owned_filename = filename.to_string();
    let owned_name = safe_name;
    let owned_world_name = body.world_name.clone();
    let owned_difficulty = body.difficulty.clone().unwrap_or_default();
    let owned_gamemode = body.gamemode.clone().unwrap_or_default();
    let owned_world_seed = body.world_seed.clone();
    let owned_banner = cfg.default_banner_color_hex.clone().unwrap_or_default();
    let plugin_template_dir = PathBuf::from(&cfg.plugin_template_dir);
    let home_dir = agent_home_dir();

    let outcome = tokio::task::spawn_blocking(move || {
        let request = CreateFromTemplateRequest {
            name: &owned_name,
            initial_world_name: owned_world_name.as_deref(),
            port,
            enable_cross_play,
            cross_play_bedrock_port,
            enable_playit,
            difficulty: &owned_difficulty,
            gamemode: &owned_gamemode,
            world_seed: owned_world_seed.as_deref(),
            default_banner_color_hex: &owned_banner,
        };
        let now = iso8601_now();
        templates::create_server_from_template(
            &StdFileSystem,
            &home_dir,
            &servers_root,
            &plugin_template_dir,
            &template_path,
            &owned_filename,
            &request,
            &now,
        )
    })
    .await;

    let created = match outcome {
        Ok(Ok(created)) => created,
        Ok(Err(error)) => return create_from_template_error_response(error),
        Err(join_error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &join_error.to_string(),
            );
        }
    };

    let server_id = created.config.id.clone();
    let server_name = created.config.display_name.clone();
    match state.register_imported_config_servers(vec![created.config], false) {
        Ok(statuses) => {
            let ready = statuses.iter().any(|(id, status)| {
                id == &server_id
                    && matches!(
                        status,
                        crate::routes::lifecycle::ReconciliationStatus::Ready
                    )
            });
            if accept_eula {
                let _ = msc_application::fleet::accept_eula(
                    &StdFileSystem,
                    &state.app_config_snapshot(),
                    &server_id,
                );
            }
            if ready {
                let _ = state.select_active_server(server_id.clone());
            }
            Json(TemplateMutationResultDto {
                success: true,
                message: format!("Created server \"{server_name}\" from template."),
                created_server_id: Some(server_id),
                created_server_name: Some(server_name),
                exported_count: None,
                templates: None,
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

fn create_from_template_error_response(
    error: msc_application::provisioning::CreateServerError,
) -> Response {
    match error {
        msc_application::provisioning::CreateServerError::EmptyName => {
            invalid_body("name_required", "name is required.")
        }
        other => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &other.to_string(),
        ),
    }
}
