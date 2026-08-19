//! `GET /v1/health` — real health-card data (P7.24), replacing the Phase 2
//! canned `demo-card`. Unlike every other route this phase wires, this
//! one runs outside the bearer-auth gate
//! (`auth-scope-phase2.md` §3, item 1).
//!
//! P7.24 also adds `GET /v1/health/problems` and `POST /v1/health/repair`,
//! reading and mutating `msc_application::diagnostics`'s
//! `last_startup_result.json` disk record for the active server —
//! `diagnostics.rs`'s own module doc calls this "the disk-fallback half"
//! it exposes [`read_last_startup_result`] for. **Flagged gap, not
//! silently worked around:** nothing in this batch calls
//! `diagnose_unexpected_stop`/`write_last_startup_result` from the real
//! `LifecycleService` stop path yet — that's a Phase 4 `lifecycle.rs`
//! integration (detecting *why* a stop happened, capturing a console
//! excerpt, deciding "reached ready state") this step's own `Files:` list
//! doesn't name, the same "no P7 step's Files: list names one" reasoning
//! `provisioning.rs`'s own `should_record_loader_version` doc already
//! used for a materially identical gap. Until that lands, these two
//! routes serve real data from a real record whenever one exists (a
//! test, a future lifecycle hook, or a hand-written file) and an honest
//! "never started" `Gray` card / empty problem list otherwise — never a
//! fabricated `ok`.

use std::path::{Path, PathBuf};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    HealthCardDto, HealthProblemsResponseDto, HealthRepairRequestDto, HealthRepairResultDto,
    HealthResponseDto, PermissionCategoryDto, StartupProblemDto,
};
use msc_application::diagnostics::{
    self, DirectoryProbe, HealthCardResult, HealthStatus, JavaCandidateProbe, RepairAction,
    RepairError,
};
use msc_domain::crash_analysis::StartupProblem;
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::StdFileSystem;

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

/// `"green"|"yellow"|"red"|"gray"` — not this route's own free choice
/// (`openapi.json` pins no enum for `HealthCardDTO.severity`/
/// `HealthResponseDTO.overallSeverity`): the already-shipped iOS
/// `HealthView.swift.severityColor(_:)` switches on exactly these four
/// literal strings (`case "red"`/`"yellow"`/`"green"`, `default` for
/// everything else including any other spelling), found by P7.26's own
/// cross-check against that file. A `"critical"/"warning"/"ok"/
/// "unknown"` vocabulary — this route's first draft — would decode fine
/// (Codable ignores nothing here, the fields still round-trip) but every
/// card would silently render in the neutral fallback color regardless
/// of real severity, exactly the "quietly dropped capability" D-023
/// warns about.
fn health_status_str(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Green => "green",
        HealthStatus::Yellow => "yellow",
        HealthStatus::Red => "red",
        HealthStatus::Gray => "gray",
    }
}

fn severity_rank(status: &str) -> u8 {
    match status {
        "red" => 3,
        "yellow" => 2,
        "gray" => 1,
        _ => 0,
    }
}

fn card_result_to_dto(card: HealthCardResult) -> HealthCardDto {
    let status = health_status_str(card.status);
    HealthCardDto {
        id: card.id.to_string(),
        title: health_card_title(card.id),
        short_label: health_card_short_label(card.id),
        severity: status.to_string(),
        detail: Some(card.detected_value),
        icon_system_name: health_card_icon(card.id, card.status),
        action_label: card.action_label.map(str::to_string),
        action_code: card.action_type.map(str::to_string),
        help_id: Some(card.help_id.to_string()),
    }
}

/// A route-layer lookup table over each card's own stable `id` — per
/// `diagnostics.rs`'s own doc on [`HealthCardResult`], the presentation
/// strings are deliberately this layer's job, not that module's.
fn health_card_title(id: &str) -> String {
    match id {
        "directory" => "Server Directory",
        "java" => "Java Runtime",
        "ram" => "RAM Allocation",
        "lastStartup" => "Last Startup",
        _ => id,
    }
    .to_string()
}

fn health_card_short_label(id: &str) -> String {
    match id {
        "directory" => "Directory",
        "java" => "Java",
        "ram" => "RAM",
        "lastStartup" => "Startup",
        _ => id,
    }
    .to_string()
}

fn health_card_icon(id: &str, status: HealthStatus) -> String {
    match id {
        "directory" => "folder",
        "java" => "cup.and.saucer",
        "ram" => "memorychip",
        "lastStartup" => match status {
            HealthStatus::Green => "checkmark.circle",
            HealthStatus::Yellow => "exclamationmark.triangle",
            HealthStatus::Red => "xmark.octagon",
            HealthStatus::Gray => "questionmark.circle",
        },
        _ => "questionmark.circle",
    }
    .to_string()
}

fn not_yet_implemented_card(id: &str, title: &str, short_label: &str) -> HealthCardDto {
    HealthCardDto {
        id: id.to_string(),
        title: title.to_string(),
        short_label: short_label.to_string(),
        severity: "gray".to_string(),
        detail: Some("Not yet implemented.".to_string()),
        icon_system_name: "questionmark.circle".to_string(),
        action_label: None,
        action_code: None,
        help_id: None,
    }
}

/// `checkDirectory`'s own caller-supplied probe (`diagnostics.rs`'s own
/// doc on [`DirectoryProbe`]): real, portable metadata reads — `writable`
/// is approximated from `Permissions::readonly()` (the one cross-platform
/// signal `std::fs` exposes), not a real `access()` probe.
fn probe_directory(dir: &Path) -> DirectoryProbe {
    match std::fs::metadata(dir) {
        Ok(meta) => DirectoryProbe {
            exists: true,
            is_dir: meta.is_dir(),
            writable: !meta.permissions().readonly(),
            readable: true,
        },
        Err(_) => DirectoryProbe {
            exists: false,
            is_dir: false,
            writable: false,
            readable: false,
        },
    }
}

/// Runs `<path> -version` and returns its combined stdout+stderr — `java
/// -version` traditionally writes to stderr, so both streams are
/// concatenated the way a captured terminal session would show them.
fn run_java_version_probe(path: &str) -> Option<(i32, String)> {
    let output = std::process::Command::new(path)
        .arg("-version")
        .output()
        .ok()?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    Some((output.status.code().unwrap_or(-1), combined))
}

/// `(candidate path, whether it exists on disk, an optional (exit code,
/// combined output) `-version` probe result)` — one row per candidate
/// Java executable, feeding [`diagnostics::check_java_runtime`].
type JavaVersionProbeRow = (String, bool, Option<(i32, String)>);

fn java_health_card(configured_java_path: &str) -> HealthCardResult {
    let mut candidate_paths: Vec<String> = Vec::new();
    if !configured_java_path.trim().is_empty() {
        candidate_paths.push(configured_java_path.trim().to_string());
    }
    candidate_paths.push("java".to_string());

    let probes: Vec<JavaVersionProbeRow> = candidate_paths
        .into_iter()
        .map(|path| {
            let exists = path.contains('/') && Path::new(&path).is_file() || !path.contains('/');
            let check = run_java_version_probe(&path);
            (path, exists, check)
        })
        .collect();

    let candidates: Vec<JavaCandidateProbe> = probes
        .iter()
        .map(|(path, exists, check)| JavaCandidateProbe {
            path,
            exists: *exists,
            version_check: check
                .as_ref()
                .map(|(code, output)| (*code, output.as_str())),
        })
        .collect();
    diagnostics::check_java_runtime(&candidates)
}

fn health_response_for(state: &LifecycleRoutesState) -> HealthResponseDto {
    let Some(server) = state.active_config_server() else {
        return HealthResponseDto {
            server_type: String::new(),
            server_name: String::new(),
            server_running: false,
            overall_severity: "gray".to_string(),
            cards: Vec::new(),
            note: Some("No active server.".to_string()),
        };
    };

    let cfg = state.app_config_snapshot();
    let directory = diagnostics::check_directory(
        &server.server_dir,
        probe_directory(Path::new(&server.server_dir)),
    );
    let java = java_health_card(&cfg.java_path);
    let ram = diagnostics::check_ram_allocation(server.max_ram_gb, detect_physical_ram_gb());
    let last_startup_record =
        diagnostics::read_last_startup_result(&StdFileSystem, Path::new(&server.server_dir));
    let started_at = last_startup_record
        .as_ref()
        .map(|record| record.started_at.clone())
        .unwrap_or_default();
    let last_startup = diagnostics::check_last_startup(last_startup_record.as_ref(), &started_at);

    let mut cards: Vec<HealthCardDto> = vec![
        card_result_to_dto(directory),
        card_result_to_dto(java),
        card_result_to_dto(ram),
        card_result_to_dto(last_startup),
        not_yet_implemented_card("portReachability", "Port Reachability", "Port"),
        not_yet_implemented_card("componentJars", "Add-on Jars", "Add-ons"),
    ];
    if server.server_type == ServerType::Bedrock {
        cards.push(not_yet_implemented_card(
            "bedrockWorldData",
            "Bedrock World Data",
            "World Data",
        ));
        cards.push(not_yet_implemented_card("vmRuntime", "VM Runtime", "VM"));
    }

    let overall_severity = cards
        .iter()
        .map(|card| card.severity.as_str())
        .max_by_key(|severity| severity_rank(severity))
        .unwrap_or("green")
        .to_string();

    HealthResponseDto {
        server_type: server.server_type.raw_value().to_string(),
        server_name: server.display_name,
        server_running: state.status_snapshot().running,
        overall_severity,
        cards,
        note: None,
    }
}

fn detect_physical_ram_gb() -> i64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            && output.status.success()
            && let Ok(text) = String::from_utf8(output.stdout)
            && let Ok(bytes) = text.trim().parse::<u64>()
        {
            return (bytes / 1_073_741_824) as i64;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    let kb: u64 = rest
                        .trim()
                        .trim_end_matches("kB")
                        .trim()
                        .parse()
                        .unwrap_or(0);
                    if kb > 0 {
                        return (kb / 1_048_576) as i64;
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["computersystem", "get", "TotalPhysicalMemory"])
            .output()
            && output.status.success()
            && let Ok(text) = String::from_utf8(output.stdout)
        {
            for line in text.lines() {
                if let Ok(bytes) = line.trim().parse::<u64>() {
                    return (bytes / 1_073_741_824) as i64;
                }
            }
        }
    }
    0
}

pub async fn health(State(state): State<LifecycleRoutesState>) -> Response {
    Json(health_response_for(&state)).into_response()
}

// ---------- GET /v1/health/problems ----------

fn startup_problem_to_dto(problem: &StartupProblem) -> StartupProblemDto {
    StartupProblemDto {
        id: problem.id(),
        kind: problem.kind.raw_value().to_string(),
        kind_title: problem.kind.title().to_string(),
        icon_system_name: problem.kind.symbol().to_string(),
        offender_name: problem.offender_name.clone(),
        requirement: problem.requirement.clone(),
        installed_file: problem.installed_file.clone(),
        installed_jar_stem: problem.installed_jar_stem.clone(),
        missing_dependency: problem.missing_dependency.clone(),
        raw_excerpt: problem.raw_excerpt.clone(),
        is_repairing: false,
        available_actions: diagnostics::available_actions(problem)
            .into_iter()
            .map(str::to_string)
            .collect(),
        modrinth_url: None,
        help_id: Some(diagnostics::crash_help_id(problem.kind).to_string()),
    }
}

fn health_problems_dto(state: &LifecycleRoutesState) -> HealthProblemsResponseDto {
    let Some(server) = state.active_config_server() else {
        return HealthProblemsResponseDto {
            server_type: String::new(),
            server_running: false,
            is_soft_fail: false,
            problems: Vec::new(),
            note: Some("No active server.".to_string()),
        };
    };
    let record =
        diagnostics::read_last_startup_result(&StdFileSystem, Path::new(&server.server_dir));
    let problems = record
        .as_ref()
        .and_then(|record| record.problems.clone())
        .unwrap_or_default();
    let is_soft_fail =
        record.as_ref().is_some_and(|record| record.was_clean) && !problems.is_empty();
    HealthProblemsResponseDto {
        server_type: server.server_type.raw_value().to_string(),
        server_running: state.status_snapshot().running,
        is_soft_fail,
        problems: problems.iter().map(startup_problem_to_dto).collect(),
        note: if record.is_some() {
            None
        } else {
            Some("No startup record yet — start this server at least once.".to_string())
        },
    }
}

pub async fn health_problems(State(state): State<LifecycleRoutesState>) -> Response {
    Json(health_problems_dto(&state)).into_response()
}

// ---------- POST /v1/health/repair ----------

pub async fn health_repair(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<HealthRepairRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if body.problem_id.trim().is_empty() {
        return invalid_body("missing_problem_id", "problemId is required.");
    }
    let action = match body.action.as_str() {
        "disable" => RepairAction::Disable,
        "delete" => RepairAction::Delete,
        "update" | "install" => {
            return invalid_body(
                "action_unavailable",
                "This repair action is not implemented yet.",
            );
        }
        _ => return invalid_body("invalid_action", "action must be disable or delete."),
    };
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "No active server.",
        );
    };
    let is_running = state.active_server_id().as_deref() == Some(server.id.as_str())
        && state.status_snapshot().running;
    if is_running {
        return error_response(StatusCode::CONFLICT, "server_running", "Server is running.");
    }

    let server_dir = Path::new(&server.server_dir);
    let record = diagnostics::read_last_startup_result(&StdFileSystem, server_dir);
    let Some(problem) = record
        .as_ref()
        .and_then(|record| record.problems.as_ref())
        .and_then(|problems| {
            problems
                .iter()
                .find(|problem| problem.id() == body.problem_id)
        })
        .cloned()
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            "problem_not_found",
            "Problem not found.",
        );
    };

    let Some(add_on_kind) = server.java_flavor.add_on_kind() else {
        return invalid_body(
            "action_unavailable",
            "This server's flavor has no add-on folder.",
        );
    };
    let add_on_dir: PathBuf = server_dir.join(add_on_kind.folder_name());

    match diagnostics::repair_problem(&StdFileSystem, &add_on_dir, &problem, action, is_running) {
        Ok(()) => Json(HealthRepairResultDto {
            success: true,
            message: "Repair applied.".to_string(),
            updated: Some(health_problems_dto(&state)),
        })
        .into_response(),
        Err(RepairError::ServerRunning) => {
            error_response(StatusCode::CONFLICT, "server_running", "Server is running.")
        }
        Err(RepairError::ActionUnavailable) => {
            invalid_body("action_unavailable", "No repair target for this problem.")
        }
        Err(RepairError::Io(error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
        Err(RepairError::VerificationFailed) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "The repair did not produce the expected on-disk state.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;

    fn admin_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "admin".to_string(),
            label: "admin".to_string(),
            role: CredentialRole::Admin,
            permissions: vec![PermissionCategoryDto::Admin],
        }
    }

    fn route_state() -> LifecycleRoutesState {
        LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        )
    }

    async fn response_body(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn call_repair(state: &LifecycleRoutesState, body: HealthRepairRequestDto) -> Response {
        health_repair(
            State(state.clone()),
            Extension(admin_credential()),
            Ok(Json(body)),
        )
        .await
    }

    #[tokio::test]
    async fn health_route_reports_no_active_server() {
        let state = route_state();
        let response = health(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_body(response).await.contains("No active server."));
    }

    #[tokio::test]
    async fn health_problems_route_reports_no_active_server() {
        let state = route_state();
        let response = health_problems(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_body(response).await.contains("No active server."));
    }

    #[tokio::test]
    async fn health_repair_route_rejects_a_blank_problem_id() {
        let state = route_state();
        let response = call_repair(
            &state,
            HealthRepairRequestDto {
                problem_id: "  ".to_string(),
                action: "disable".to_string(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response).await.contains("missing_problem_id"));
    }

    #[tokio::test]
    async fn health_repair_route_rejects_an_invalid_action() {
        let state = route_state();
        let response = call_repair(
            &state,
            HealthRepairRequestDto {
                problem_id: "p1".to_string(),
                action: "reinstall".to_string(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response).await.contains("invalid_action"));
    }

    #[tokio::test]
    async fn health_repair_route_reports_update_and_install_as_unavailable() {
        let state = route_state();
        for action in ["update", "install"] {
            let response = call_repair(
                &state,
                HealthRepairRequestDto {
                    problem_id: "p1".to_string(),
                    action: action.to_string(),
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(response_body(response).await.contains("action_unavailable"));
        }
    }

    #[tokio::test]
    async fn health_repair_route_reports_no_active_server() {
        let state = route_state();
        let response = call_repair(
            &state,
            HealthRepairRequestDto {
                problem_id: "p1".to_string(),
                action: "disable".to_string(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response_body(response).await.contains("no_active_server"));
    }
}
