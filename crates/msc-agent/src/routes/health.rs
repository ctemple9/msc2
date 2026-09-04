//! `GET /v1/health` — real health-card data (P7.24), replacing the Phase 2
//! canned `demo-card`. Unlike every other route this phase wires, this
//! one runs outside the bearer-auth gate
//! (`auth-scope-phase2.md` §3, item 1).
//!
//! P7.24 also adds `GET /v1/health/problems` and `POST /v1/health/repair`,
//! reading and mutating `msc_application::diagnostics`'s
//! `last_startup_result.json` disk record for the active server —
//! `diagnostics.rs`'s own module doc calls this "the disk-fallback half"
//! it exposes [`read_last_startup_result`] for. **P7.32 closed the gap
//! this doc used to flag here:** `LifecycleService::mark_process_exited`
//! (`msc-application/src/lifecycle.rs`) now calls
//! `diagnose_unexpected_stop`/`write_last_startup_result` for real, on
//! every process exit the real `LifecycleService` stop path sees — not
//! just from a test or a hand-written file. These two routes read
//! whatever that real write produced (or an honest "never started" `Gray`
//! card / empty problem list before the server has ever run) — never a
//! fabricated `ok`. **P7.36 closed the gap this doc used to flag here:**
//! `msc_application::add_on_inventory::scan_mods`/`scan_plugins` now feed
//! real `installed_mods`/`installed_plugins` into both
//! `diagnose_unexpected_stop` (hard fails, `lifecycle.rs`'s
//! `record_stop_diagnostics`) and `scan_paper_soft_failures` (Paper plugin
//! soft fails on a successful start, `lifecycle.rs`'s `mark_ready`) — a
//! problem naming an installed jar now attributes `installed_jar_stem`
//! for real, which is what actually turns on the disable/delete repair
//! actions [`diagnostics::available_actions`] offers. `health_repair`
//! below also now calls [`diagnostics::remove_repaired_problem`] after a
//! verified repair, so a re-read of this same route doesn't keep
//! reporting a problem that's already been fixed — MSC 1 never needed
//! this (it drops the repaired problem from a session-local in-memory
//! array instead; this headless agent has no such cache and reads the
//! persisted record fresh every call).

use std::path::{Path, PathBuf};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    BedrockBackendDto, BedrockRuntimeStateDto, HealthCardDto, HealthProblemsResponseDto,
    HealthRepairRequestDto, HealthRepairResultDto, HealthResponseDto, PermissionCategoryDto,
    StartupProblemDto,
};
use msc_application::diagnostics::{
    self, DirectoryProbe, HealthCardResult, HealthStatus, JavaCandidateProbe, RepairAction,
    RepairError,
};
use msc_domain::crash_analysis::StartupProblem;
use msc_domain::identity::ServerType;
use msc_domain::networking::DiagnosticResult;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::port_diagnostics::{probe_tcp, probe_udp};

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
        "componentJars" => "Add-on Jars",
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
        "componentJars" => "Add-ons",
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
        "componentJars" => match status {
            HealthStatus::Green => "checkmark.seal",
            HealthStatus::Yellow => "exclamationmark.triangle",
            HealthStatus::Red => "xmark.seal",
            HealthStatus::Gray => "questionmark.circle",
        },
        _ => "questionmark.circle",
    }
    .to_string()
}

fn bedrock_vm_runtime_card(runtime: BedrockRuntimeStateDto) -> HealthCardDto {
    let state = runtime.state.as_str();
    let fallback_detail = runtime
        .message
        .unwrap_or_else(|| format!("Bedrock runtime state: {state}."));
    let (severity, detail) = match (state, runtime.backend) {
        ("available", Some(BedrockBackendDto::Native)) => (
            "green",
            "Native Bedrock runtime is available; no VM is required.".to_string(),
        ),
        ("available", Some(BedrockBackendDto::VzSidecar)) => {
            ("green", "Bedrock VM sidecar is available.".to_string())
        }
        ("available", None) => ("green", fallback_detail),
        ("provisioning_required", _) => ("yellow", fallback_detail),
        ("unavailable", _) => ("gray", fallback_detail),
        (_, _) => ("gray", fallback_detail),
    };

    HealthCardDto {
        id: "vmRuntime".to_string(),
        title: "VM Runtime".to_string(),
        short_label: "VM".to_string(),
        severity: severity.to_string(),
        detail: Some(detail),
        icon_system_name: "memorychip".to_string(),
        action_label: None,
        action_code: None,
        help_id: runtime.help_id,
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

fn server_port(server: &msc_domain::app_config_schema::ConfigServer) -> u16 {
    if server.server_type == ServerType::Bedrock {
        return server
            .bedrock_port
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port > 0)
            .unwrap_or(19132);
    }

    std::fs::read_to_string(Path::new(&server.server_dir).join("server.properties"))
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find_map(|line| line.strip_prefix("server-port="))
                .and_then(|value| value.trim().parse::<u16>().ok())
        })
        .filter(|port| *port > 0)
        .unwrap_or(25565)
}

fn port_reachability_card(
    server: &msc_domain::app_config_schema::ConfigServer,
    running: bool,
    local: DiagnosticResult,
) -> HealthCardDto {
    let port = server_port(server);
    let protocol = if server.server_type == ServerType::Bedrock {
        "UDP"
    } else {
        "TCP"
    };
    let (detail, severity, action_label, action_code) = if !server.has_ever_started && !running {
        (
            format!("Waiting for first start\nPort {} ({})", port, protocol),
            "gray",
            Some("View Port Setup Guide"),
            Some("openRouterPortForwardGuide"),
        )
    } else if !running {
        (
            format!(
                "Server is off\nStart it to verify port {} ({}) reachability.",
                port, protocol
            ),
            "gray",
            Some("View Port Setup Guide"),
            Some("openRouterPortForwardGuide"),
        )
    } else {
        match local {
            DiagnosticResult::Open => (
                format!(
                    "Port {} ({})\nListening locally\nPublic access unverified.",
                    port, protocol
                ),
                "yellow",
                None,
                None,
            ),
            DiagnosticResult::Closed => (
                format!(
                    "Port {} ({})\nServer is running but nothing is listening on this port.\nCheck the port in your server settings.",
                    port, protocol
                ),
                "red",
                Some("View Port Setup Guide"),
                Some("openRouterPortForwardGuide"),
            ),
            DiagnosticResult::Unreachable | DiagnosticResult::Unavailable => (
                format!(
                    "Port {} ({})\nThe local listener could not be verified.\nSee Networking for more diagnostics.",
                    port, protocol
                ),
                "yellow",
                None,
                None,
            ),
            DiagnosticResult::NotAttempted | DiagnosticResult::NotApplicable => (
                format!("Port {} ({})\nNo probe was attempted.", port, protocol),
                "gray",
                None,
                None,
            ),
        }
    };

    HealthCardDto {
        id: "portReachability".to_string(),
        title: "Port Reachability".to_string(),
        short_label: "Port".to_string(),
        severity: severity.to_string(),
        detail: Some(detail),
        icon_system_name: "network".to_string(),
        action_label: action_label.map(str::to_string),
        action_code: action_code.map(str::to_string),
        help_id: None,
    }
}

async fn health_response_for(state: &LifecycleRoutesState) -> HealthResponseDto {
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
    let server_running = state.status_snapshot().running;
    let port = server_port(&server);
    let server_type = server.server_type;
    let local_port = if server_running {
        tokio::task::spawn_blocking(move || {
            if server_type == ServerType::Bedrock {
                probe_udp("127.0.0.1", port, std::time::Duration::from_secs(1))
            } else {
                probe_tcp("127.0.0.1", port, std::time::Duration::from_secs(1))
            }
        })
        .await
        .unwrap_or(DiagnosticResult::Unavailable)
    } else {
        DiagnosticResult::NotAttempted
    };
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
    let add_on_kind = server.java_flavor.add_on_kind();
    let installed_count = add_on_kind
        .map(|kind| {
            let add_on_dir = Path::new(&server.server_dir).join(kind.folder_name());
            match kind {
                msc_domain::identity::AddOnKind::Mod => {
                    msc_application::add_on_inventory::scan_mods(&StdFileSystem, &add_on_dir).len()
                }
                msc_domain::identity::AddOnKind::Plugin => {
                    msc_application::add_on_inventory::scan_plugins(&StdFileSystem, &add_on_dir)
                        .len()
                }
            }
        })
        .unwrap_or(0);
    let problems = last_startup_record
        .as_ref()
        .and_then(|record| record.problems.clone())
        .unwrap_or_default();
    let component_jars = diagnostics::check_component_jars(add_on_kind, installed_count, &problems);

    let mut cards: Vec<HealthCardDto> = vec![
        card_result_to_dto(directory),
        card_result_to_dto(java),
        card_result_to_dto(ram),
        card_result_to_dto(last_startup),
        port_reachability_card(&server, server_running, local_port),
        card_result_to_dto(component_jars),
    ];
    if server.server_type == ServerType::Bedrock {
        cards.push(bedrock_vm_runtime_card(state.bedrock_runtime_state()));
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
        server_running,
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
    let mut response = Json(health_response_for(&state).await).into_response();
    // This route already runs outside the bearer-auth gate (see module doc)
    // and carries no secrets, so a permissive CORS allowance is safe here —
    // unlike every authenticated route, which stays same-origin-only. A
    // Tauri window's own webview loads its dev-mode UI from a plain
    // `http://` devUrl origin distinct from the agent's, and the shell's
    // pre-credential readiness probe (`localAgentHealthCheck`,
    // `clients/desktop-web/src/lib/platform/index.ts`) uses a bare browser
    // `fetch()` to this one route before any native-bridge credential
    // exists — without this header that probe is silently CORS-blocked and
    // the shell never gets past "Agent starting" in a dev-mode desktop
    // build, even though the agent is actually healthy.
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_static("*"),
    );
    response
}

/// Cheap unauthenticated liveness response for desktop startup and reconnect.
/// The full `/health` route is intentionally richer, but its diagnostics may
/// inspect every installed mod JAR and therefore is not a service readiness
/// probe.
pub async fn healthz() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_static("*"),
    );
    response
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

/// P8.23: `update`/`install` join `disable`/`delete` as real repair kinds
/// — see `msc_application::addon_updates`'s own
/// `repair_update`/`repair_install_missing_dependency` doc for what each
/// actually does. Both are real network-touching mutations (a resolve
/// pass, a Modrinth search), appropriate here since this whole route sits
/// behind the `Settings` permission — unlike `GET /v1/health`'s own
/// `componentJars` card, which `diagnostics.rs`'s doc explains stays
/// offline on purpose.
enum RepairKind {
    Disable,
    Delete,
    Update,
    Install,
}

fn health_repair_error_response(error: &RepairFailure) -> Response {
    use msc_application::addon_updates::HealthRepairError;
    match error {
        RepairFailure::Diagnostics(RepairError::ServerRunning)
        | RepairFailure::Update(HealthRepairError::ServerRunning) => {
            error_response(StatusCode::CONFLICT, "server_running", "Server is running.")
        }
        RepairFailure::Diagnostics(RepairError::ActionUnavailable)
        | RepairFailure::Update(HealthRepairError::NoAddOnKind)
        | RepairFailure::Update(HealthRepairError::ActionUnavailable) => {
            invalid_body("action_unavailable", "No repair target for this problem.")
        }
        RepairFailure::Diagnostics(RepairError::Io(e)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        ),
        RepairFailure::Diagnostics(RepairError::VerificationFailed) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "The repair did not produce the expected on-disk state.",
        ),
        RepairFailure::Update(HealthRepairError::NoUpdateAvailable) => invalid_body(
            "no_update_available",
            "No update is available for this add-on.",
        ),
        RepairFailure::Update(HealthRepairError::NoConfidentMatch) => invalid_body(
            "no_confident_match",
            "Could not confidently identify this dependency on Modrinth.",
        ),
        RepairFailure::Update(HealthRepairError::Mutation(
            msc_application::addons::AddonMutationError::PackManaged,
        )) => error_response(
            StatusCode::CONFLICT,
            "conflict",
            "This server is managed by a modpack.",
        ),
        RepairFailure::Update(HealthRepairError::Mutation(mutation_error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &mutation_error.to_string(),
        ),
        RepairFailure::JoinFailed => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "The repair task failed to run to completion.",
        ),
    }
}

enum RepairFailure {
    Diagnostics(RepairError),
    Update(msc_application::addon_updates::HealthRepairError),
    JoinFailed,
}

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
    let kind = match body.action.as_str() {
        "disable" => RepairKind::Disable,
        "delete" => RepairKind::Delete,
        "update" => RepairKind::Update,
        "install" => RepairKind::Install,
        _ => {
            return invalid_body(
                "invalid_action",
                "action must be disable, delete, update, or install.",
            );
        }
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

    let repair_outcome: Result<(), RepairFailure> = match kind {
        RepairKind::Disable | RepairKind::Delete => {
            let action = if matches!(kind, RepairKind::Disable) {
                RepairAction::Disable
            } else {
                RepairAction::Delete
            };
            diagnostics::repair_problem(&StdFileSystem, &add_on_dir, &problem, action, is_running)
                .map_err(RepairFailure::Diagnostics)
        }
        RepairKind::Update | RepairKind::Install => {
            let server = server.clone();
            let problem = problem.clone();
            let join_result = tokio::task::spawn_blocking(move || {
                use msc_infrastructure::addon_provider::HttpTransport;
                use msc_infrastructure::fs::StdFileSystem as Fs;
                let transport = HttpTransport::new();
                let server_dir = Path::new(&server.server_dir);
                if matches!(kind, RepairKind::Update) {
                    msc_application::addon_updates::repair_update(
                        &transport,
                        &Fs,
                        server_dir,
                        server.java_flavor,
                        server.minecraft_version.as_deref(),
                        &server.addon_links.clone().unwrap_or_default(),
                        &server.plugin_sources.clone().unwrap_or_default(),
                        server.pack_managed,
                        &problem,
                        is_running,
                        &|| false,
                    )
                    .map(|_| ())
                } else {
                    msc_application::addon_updates::repair_install_missing_dependency(
                        &transport,
                        &Fs,
                        server_dir,
                        server.java_flavor,
                        server.minecraft_version.as_deref(),
                        server.pack_managed,
                        &problem,
                        is_running,
                        &|| false,
                    )
                    .map(|_| ())
                }
            })
            .await;
            match join_result {
                Ok(inner) => inner.map_err(RepairFailure::Update),
                Err(_) => Err(RepairFailure::JoinFailed),
            }
        }
    };

    match repair_outcome {
        Ok(()) => {
            // P7.36: MSC 1 keeps a session-local `startupProblems` array
            // and drops the repaired one there; this agent has no such
            // cache, so `last_startup_result.json` itself must lose the
            // problem or a fresh read here (and every read after) would
            // keep reporting a repair that already verified as done.
            match diagnostics::remove_repaired_problem(&StdFileSystem, server_dir, &body.problem_id)
            {
                Ok(true) => {}
                Ok(false) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal_error",
                        "The repair succeeded on disk, but its problem record was not updated.",
                    );
                }
                Err(error) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal_error",
                        &format!(
                            "The repair succeeded on disk, but its problem record could not be saved: {error}"
                        ),
                    );
                }
            }
            Json(HealthRepairResultDto {
                success: true,
                message: "Repair applied.".to_string(),
                operation_id: None,
                updated: Some(health_problems_dto(&state)),
            })
            .into_response()
        }
        Err(ref failure) => health_repair_error_response(failure),
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

    #[test]
    fn bedrock_vm_runtime_card_reflects_backend_state() {
        let native = bedrock_vm_runtime_card(BedrockRuntimeStateDto {
            state: "available".to_string(),
            backend: Some(BedrockBackendDto::Native),
            host_os: None,
            reason_code: None,
            message: None,
            help_id: None,
        });
        assert_eq!(native.severity, "green");
        assert_eq!(
            native.detail.as_deref(),
            Some("Native Bedrock runtime is available; no VM is required.")
        );

        let sidecar = bedrock_vm_runtime_card(BedrockRuntimeStateDto {
            state: "available".to_string(),
            backend: Some(BedrockBackendDto::VzSidecar),
            host_os: None,
            reason_code: None,
            message: None,
            help_id: None,
        });
        assert_eq!(sidecar.severity, "green");
        assert_eq!(
            sidecar.detail.as_deref(),
            Some("Bedrock VM sidecar is available.")
        );

        let provisioning = bedrock_vm_runtime_card(BedrockRuntimeStateDto {
            state: "provisioning_required".to_string(),
            backend: Some(BedrockBackendDto::Native),
            host_os: None,
            reason_code: Some("archive_missing".to_string()),
            message: Some("A verified Bedrock distribution is required before start.".to_string()),
            help_id: None,
        });
        assert_eq!(provisioning.severity, "yellow");
        assert_eq!(
            provisioning.detail.as_deref(),
            Some("A verified Bedrock distribution is required before start.")
        );
    }

    #[tokio::test]
    async fn health_route_reports_no_active_server() {
        let state = route_state();
        let response = health(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_body(response).await.contains("No active server."));
    }

    #[tokio::test]
    async fn healthz_route_is_a_cheap_liveness_probe() {
        let response = healthz().await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()["access-control-allow-origin"], "*");
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
    async fn health_repair_route_accepts_update_and_install_as_valid_actions() {
        // P8.23: `update`/`install` are now real repair kinds, not a
        // hardcoded `action_unavailable` — with no active server, both
        // reach the same `no_active_server` guard `disable`/`delete`
        // already do, proving the action itself parsed successfully
        // rather than being rejected by the `invalid_action` guard.
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
            assert_eq!(response.status(), StatusCode::CONFLICT);
            assert!(response_body(response).await.contains("no_active_server"));
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
