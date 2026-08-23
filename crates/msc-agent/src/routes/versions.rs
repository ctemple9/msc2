//! P7.24: `GET /v1/versions`, `GET /v1/versions/create`,
//! `POST /v1/components/version`, `GET /v1/java-runtimes`,
//! `GET`/`POST /v1/config/java-runtime`, `GET`/`POST /v1/config/ram`, and
//! `POST /v1/java-runtimes/install` — real network/process-backed Java
//! runtime and server-version management over `msc_application::
//! server_versions` and `msc_infrastructure::{jar_provider,
//! java_runtime_detection, java_runtime_install}`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    JavaConfigResponseDto, JavaConfigSetRequestDto, JavaRuntimeDto, JavaRuntimeInstallRequestDto,
    JavaRuntimeInstallResultDto, JavaRuntimesResponseDto, PermissionCategoryDto,
    RamConfigResponseDto, RamConfigUpdateRequestDto, RamConfigUpdateResultDto,
    VersionChangeRequestDto, VersionChangeResultDto, VersionEntryDto, VersionsResponseDto,
};
use msc_application::server_versions::{
    self, ChangeVersionError, ChangeVersionRequest, VersionListEntry,
};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use msc_domain::java_runtime::MINECRAFT_INSTALL_OPTIONS;
use msc_domain::world::BackupAssociation;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::jar_provider::HttpTransport;
use msc_infrastructure::java_runtime_detection::{
    self, DetectedJavaRuntime, HostOs, default_java_runtime_search_roots,
};
use msc_infrastructure::java_runtime_install::{self, AdoptiumAsset, JavaRuntimeInstallError};

use crate::auth::AuthenticatedCredential;
use crate::routes::bedrock::{require_runtime, runtime_for};
use crate::routes::lifecycle::{
    LifecycleRoutesState, TryMutateError, error_response, invalid_body, require_permission,
};
use crate::routes::operations::operation_error_response;

const INSTALLER_TIMEOUT: Duration = Duration::from_secs(10 * 60);

fn agent_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn current_host_os() -> HostOs {
    #[cfg(target_os = "macos")]
    {
        HostOs::Mac
    }
    #[cfg(target_os = "linux")]
    {
        HostOs::Linux
    }
    #[cfg(target_os = "windows")]
    {
        HostOs::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        HostOs::Linux
    }
}

/// Adoptium's own `architecture=` query parameter spelling
/// (`x64`/`aarch64`/`x86`) — `std::env::consts::ARCH` uses Rust's own
/// target-triple vocabulary, which differs for the one architecture this
/// agent actually ships on today (`x86_64` -> `x64`).
fn current_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        "x86" => "x86",
        other => other,
    }
}

fn runtimes_root() -> PathBuf {
    msc_infrastructure::config_repository::default_app_data_dir().join("runtimes")
}

fn version_entry_to_dto(entry: VersionListEntry) -> VersionEntryDto {
    let is_latest = entry.entry.is_latest();
    VersionEntryDto {
        id: entry.entry.id,
        display_label: entry.entry.display_label,
        mc_version: entry.entry.mc_version,
        loader_version: entry.entry.loader_version,
        build_label: entry.entry.build_label,
        is_stable: entry.entry.is_stable,
        is_latest,
    }
}

/// `false` for the three flavors MSC 1's own create flow never offers a
/// version picker for (`ServerJarProvider.listVersions`'s `default:
/// return []`, `server_versions::list_versions_for_server`'s own doc).
fn flavor_supports_versions(flavor: JavaServerFlavor) -> bool {
    !matches!(
        flavor,
        JavaServerFlavor::Pufferfish | JavaServerFlavor::Spigot | JavaServerFlavor::Quilt
    )
}

/// Shared by `GET /v1/versions` (a real, already-registered server) and
/// `GET /v1/versions/create` (a hypothetical one, `current_version:
/// None`) — real network fetch, run off the async executor thread.
/// Degrades honestly on a provider failure per this phase's own working
/// exit criteria: `supports_versions` stays true (the flavor genuinely
/// has a picker), `versions` comes back empty, and `note` explains why,
/// rather than fabricating a version list or 500ing the whole route.
async fn fetch_versions_response(
    flavor: JavaServerFlavor,
    current_version: Option<String>,
) -> VersionsResponseDto {
    if !flavor_supports_versions(flavor) {
        return VersionsResponseDto {
            supports_versions: false,
            flavor_name: flavor.raw_value().to_string(),
            current_version,
            is_bedrock: false,
            versions: Vec::new(),
            note: Some(format!(
                "{} is not offered a version picker.",
                flavor.raw_value()
            )),
            runtime: None,
        };
    }
    let current_for_fetch = current_version.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let transport = HttpTransport::new();
        server_versions::list_versions_for_server(&transport, flavor, current_for_fetch.as_deref())
    })
    .await;
    match outcome {
        Ok(Ok(entries)) => VersionsResponseDto {
            supports_versions: true,
            flavor_name: flavor.raw_value().to_string(),
            current_version,
            is_bedrock: false,
            versions: entries.into_iter().map(version_entry_to_dto).collect(),
            note: None,
            runtime: None,
        },
        Ok(Err(error)) => VersionsResponseDto {
            supports_versions: true,
            flavor_name: flavor.raw_value().to_string(),
            current_version,
            is_bedrock: false,
            versions: Vec::new(),
            note: Some(format!("Could not fetch versions: {error}")),
            runtime: None,
        },
        Err(join_error) => VersionsResponseDto {
            supports_versions: true,
            flavor_name: flavor.raw_value().to_string(),
            current_version,
            is_bedrock: false,
            versions: Vec::new(),
            note: Some(format!("Could not fetch versions: {join_error}")),
            runtime: None,
        },
    }
}

fn bedrock_versions_response(
    current_version: Option<String>,
    runtime: Option<msc_api::dto::BedrockRuntimeStateDto>,
) -> VersionsResponseDto {
    let verified_version = runtime
        .as_ref()
        .and_then(|runtime| (runtime.state == "available").then(|| current_version.clone())?);
    let versions: Vec<VersionEntryDto> = verified_version
        .as_ref()
        .map(|version| VersionEntryDto {
            id: version.clone(),
            display_label: format!("Bedrock {version}"),
            mc_version: version.clone(),
            loader_version: None,
            build_label: None,
            is_stable: true,
            is_latest: true,
        })
        .into_iter()
        .collect();
    VersionsResponseDto {
        supports_versions: !versions.is_empty(),
        flavor_name: "bedrock".to_string(),
        current_version,
        is_bedrock: true,
        versions,
        note: Some(
            "Bedrock versions are limited to the verified distribution selected for this runtime."
                .to_string(),
        ),
        runtime,
    }
}

// ---------- GET /v1/versions ----------

pub async fn versions(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return axum::Json(VersionsResponseDto {
            supports_versions: false,
            flavor_name: String::new(),
            current_version: None,
            is_bedrock: false,
            versions: Vec::new(),
            note: Some("No active server.".to_string()),
            runtime: None,
        })
        .into_response();
    };
    if server.server_type == ServerType::Bedrock {
        return axum::Json(bedrock_versions_response(
            server.minecraft_version.clone(),
            runtime_for(&state),
        ))
        .into_response();
    }
    axum::Json(fetch_versions_response(server.java_flavor, server.minecraft_version.clone()).await)
        .into_response()
}

// ---------- GET /v1/versions/create ----------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionsCreateQuery {
    #[serde(default)]
    server_type: Option<String>,
    #[serde(default)]
    java_flavor: Option<String>,
}

pub async fn versions_for_create(Query(query): Query<VersionsCreateQuery>) -> Response {
    let server_type = query.server_type.as_deref().unwrap_or("java");
    if server_type.eq_ignore_ascii_case("bedrock") {
        return axum::Json(bedrock_versions_response(None, None)).into_response();
    }
    let flavor = query
        .java_flavor
        .as_deref()
        .and_then(JavaServerFlavor::from_raw_value)
        .unwrap_or(JavaServerFlavor::Paper);
    axum::Json(fetch_versions_response(flavor, None).await).into_response()
}

// ---------- POST /v1/components/version ----------

pub async fn change_version(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<axum::Json<VersionChangeRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let axum::Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if body.version_id.trim().is_empty() {
        return invalid_body("missing_version_id", "versionId is required.");
    }
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "No active server.",
        );
    };
    if server.server_type == ServerType::Bedrock {
        if let Some(response) = require_runtime(&state) {
            return response;
        }
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "Bedrock version changes require a verified distribution manifest.",
        );
    }
    if server.server_type != ServerType::Java
        || matches!(
            server.java_flavor,
            JavaServerFlavor::Pufferfish | JavaServerFlavor::Spigot | JavaServerFlavor::Quilt
        )
    {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "This server's flavor does not support version changes.",
        );
    }
    if state.active_server_id().as_deref() == Some(server.id.as_str())
        && state.status_snapshot().running
    {
        return error_response(StatusCode::CONFLICT, "server_running", "Server is running.");
    }

    let operation_id = match state.operations().begin_lifecycle(
        "version-change",
        Some(server.id.clone()),
        "Changing server version.",
    ) {
        Ok(id) => id,
        Err(msc_application::operations::LifecycleOperationError::Conflict(error)) => {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "download_in_progress",
                &error.message,
            );
        }
        Err(error) => return operation_error_response(error),
    };

    let cfg = state.app_config_snapshot();
    let java_path = cfg.java_path.clone();
    let version_id = body.version_id.clone();
    let loader_version = body.loader_version.clone();
    let server_dir = PathBuf::from(&server.server_dir);
    let paper_jar_path = server.paper_jar_path.clone();
    let current_minecraft_version = server.minecraft_version.clone();
    let flavor = server.java_flavor;
    let server_id = server.id.clone();
    let server_type = server.server_type;

    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_change_version(
                worker_state,
                worker_operation_id,
                server_id,
                server_type,
                flavor,
                java_path,
                version_id,
                loader_version,
                server_dir,
                paper_jar_path,
                current_minecraft_version,
            )
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    axum::Json(VersionChangeResultDto {
        success: true,
        message: "Version change started.".to_string(),
        requires_restart: true,
        operation_id: Some(operation_id.as_str().to_string()),
        runtime: None,
    })
    .into_response()
}

/// The `pre_downgrade_backup` closure `change_version` calls only when
/// [`msc_domain::version::is_downgrade`] says the target is genuinely
/// older than what's installed — the same composition `routes/worlds.rs`'s
/// own private `run_pre_mutation_safety_backup` already establishes for
/// world-conversion, duplicated here per that file's own "genuinely
/// disk-shaped work" precedent rather than exposed across a module
/// boundary for one more caller.
fn pre_downgrade_backup(
    server_dir: &Path,
    server_type: ServerType,
    should_cancel: &dyn Fn() -> bool,
) -> bool {
    let now = iso8601_now();
    let association = BackupAssociation {
        slot_id: None,
        slot_name: None,
        world_seed: None,
    };
    msc_application::backups::create_backup(
        &StdFileSystem,
        server_dir,
        server_type,
        None,
        &association,
        None,
        None,
        false,
        true,
        Some("pre-downgrade"),
        None,
        &now,
        None,
        || false,
        should_cancel,
    )
    .is_ok()
}

fn iso8601_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86_400;
    let remainder = secs % 86_400;
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

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

#[allow(clippy::too_many_arguments)]
fn run_change_version(
    state: LifecycleRoutesState,
    operation_id: msc_domain::operation::OperationId,
    server_id: String,
    server_type: ServerType,
    flavor: JavaServerFlavor,
    java_path: String,
    version_id: String,
    loader_version: Option<String>,
    server_dir: PathBuf,
    paper_jar_path: String,
    current_minecraft_version: Option<String>,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state
            .operations()
            .cancel(&operation_id, "Version change cancelled before it started.");
        return;
    }
    let transport = HttpTransport::new();
    let supervisor = state.process_supervisor();
    let request = ChangeVersionRequest {
        flavor,
        version_id: &version_id,
        loader_version: loader_version.as_deref(),
        current_minecraft_version: current_minecraft_version.as_deref(),
        server_dir: &server_dir,
        paper_jar_path: &paper_jar_path,
    };
    let backup_server_dir = server_dir.clone();
    let backup_should_cancel = should_cancel.clone();
    let result = server_versions::change_version(
        &StdFileSystem,
        &transport,
        supervisor,
        &java_path,
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        || pre_downgrade_backup(&backup_server_dir, server_type, &backup_should_cancel),
        &should_cancel,
        |_stream, _bytes| {},
    );

    match result {
        Ok(changed) => {
            let update_result = state.try_mutate_config(|config| {
                let server = config
                    .servers
                    .iter_mut()
                    .find(|s| s.id == server_id)
                    .ok_or(())?;
                server.minecraft_version = Some(changed.minecraft_version.clone());
                server.server_build = Some(changed.build.clone());
                if matches!(
                    flavor,
                    JavaServerFlavor::Fabric | JavaServerFlavor::NeoForge | JavaServerFlavor::Forge
                ) && let Some(loader) = &changed.loader_version
                {
                    server.loader_version = Some(loader.clone());
                }
                Ok::<(), ()>(())
            });
            match update_result {
                Ok(()) => {
                    let mut result_map = std::collections::BTreeMap::new();
                    result_map.insert(
                        "minecraftVersion".to_string(),
                        changed.minecraft_version.clone(),
                    );
                    result_map.insert("build".to_string(), changed.build.clone());
                    let _ = state.finish_operation_success(
                        &operation_id,
                        &format!("Changed version to {}.", changed.minecraft_version),
                        result_map,
                    );
                }
                Err(_) => {
                    let _ = state.finish_operation_failure(
                        &operation_id,
                        "internal_error",
                        "server was removed while its version change was running".to_string(),
                    );
                }
            }
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                change_version_error_code(&error),
                error.to_string(),
            );
        }
    }
}

fn change_version_error_code(error: &ChangeVersionError) -> &'static str {
    match error {
        ChangeVersionError::ServerRunning => "server_running",
        ChangeVersionError::DownloadInProgress => "download_in_progress",
        ChangeVersionError::UnsupportedFlavor(_) => "not_supported",
        ChangeVersionError::Cancelled => "cancelled",
        _ => "internal_error",
    }
}

// ---------- GET /v1/java-runtimes ----------

pub async fn java_runtimes(State(state): State<LifecycleRoutesState>) -> Response {
    let home_dir = agent_home_dir();
    let host = current_host_os();
    let detected = tokio::task::spawn_blocking(move || {
        let mut roots = vec![runtimes_root().to_string_lossy().into_owned()];
        roots.extend(default_java_runtime_search_roots(host, &home_dir));
        java_runtime_detection::detect_installed_java_runtimes(&StdFileSystem, &roots)
    })
    .await
    .unwrap_or_default();

    // P7.31: corroborate each entry's path-inferred `major_version` (a
    // regex-over-directory-name guess) with a real probe -- the same
    // mechanism the required-major guard itself runs against the
    // *configured* runtime, applied here to every *discovered* one.
    let supervisor = state.process_supervisor();
    let runtimes = tokio::task::spawn_blocking(move || {
        detected
            .into_iter()
            .map(|runtime| with_probed_major_version(supervisor, runtime))
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    axum::Json(JavaRuntimesResponseDto {
        runtimes: runtimes.into_iter().map(detected_runtime_to_dto).collect(),
    })
    .into_response()
}

/// A probe failure (spawn error, unparseable banner) leaves the path-
/// inferred guess in place rather than blanking out a value this route
/// already had -- the probe only ever *improves* on the guess, never
/// regresses to less information than before.
fn with_probed_major_version(
    supervisor: &'static (dyn msc_infrastructure::process::ProcessSupervisor + Send + Sync),
    mut runtime: DetectedJavaRuntime,
) -> DetectedJavaRuntime {
    let probe =
        java_runtime_detection::run_java_version_probe(supervisor, &runtime.executable_path);
    if let msc_domain::java_runtime::JavaVersionProbe::Captured { output } = &probe
        && let Some(major) = msc_domain::java_runtime::parse_major(output)
    {
        runtime.major_version = Some(major);
    }
    runtime
}

fn detected_runtime_to_dto(runtime: DetectedJavaRuntime) -> JavaRuntimeDto {
    JavaRuntimeDto {
        name: runtime.name,
        executable_path: runtime.executable_path,
        major_version: runtime.major_version,
    }
}

// ---------- GET/POST /v1/config/java-runtime ----------

pub async fn get_java_config(State(state): State<LifecycleRoutesState>) -> Response {
    let cfg = state.app_config_snapshot();
    axum::Json(JavaConfigResponseDto {
        executable_path: Some(cfg.java_path),
    })
    .into_response()
}

pub async fn set_java_config(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<axum::Json<JavaConfigSetRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let axum::Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let path = msc_domain::java_runtime::resolved_settings_java_path(
        body.executable_path.as_deref().unwrap_or("").trim(),
    );
    let result = state.try_mutate_config(|config| {
        config.java_path = path.clone();
        Ok::<(), ()>(())
    });
    match result {
        Ok(()) => {
            let cfg = state.app_config_snapshot();
            axum::Json(JavaConfigResponseDto {
                executable_path: Some(cfg.java_path),
            })
            .into_response()
        }
        Err(TryMutateError::Domain(())) => unreachable!("set_java_config's update never fails"),
        Err(TryMutateError::Save(error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "set_failed",
            &error.to_string(),
        ),
    }
}

// ---------- GET/POST /v1/config/ram ----------

/// Best-effort physical-RAM probe, shelling out to each platform's own
/// tool the way `metrics.rs`'s own `ps`-backed `PsProcessMetricsProvider`
/// already does for process RSS — no `sysinfo`-style crate is a
/// dependency of this workspace today, and a plain memory-size read
/// doesn't justify adding one. `0` on any failure or an unrecognized
/// platform, which `check_ram_allocation`'s own domain logic already
/// treats as "skip the physical-RAM-relative checks" rather than a crash
/// — the same honest-degradation shape this phase's version-listing
/// routes already use for a provider outage.
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

/// No fixture pins an exact recommendation; a documented, reasonable
/// default (leave 1GB headroom for the host OS) matching this crate's
/// own `INTERVAL_OPTIONS_MINUTES` precedent (`routes/backups.rs`) for
/// "a P7.24 wiring decision, not an oracle value."
fn recommended_max_ram_gb(physical_gb: i64) -> i64 {
    if physical_gb <= 1 {
        physical_gb
    } else {
        physical_gb - 1
    }
}

fn no_active_server_ram_response() -> Response {
    axum::Json(RamConfigResponseDto {
        server_name: String::new(),
        server_type: String::new(),
        min_ram_gb: 0.0,
        max_ram_gb: 0.0,
        physical_ram_gb: detect_physical_ram_gb(),
        recommended_max_gb: recommended_max_ram_gb(detect_physical_ram_gb()),
        server_running: false,
        has_active_server: false,
    })
    .into_response()
}

pub async fn get_ram_config(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return no_active_server_ram_response();
    };
    let physical = detect_physical_ram_gb();
    axum::Json(RamConfigResponseDto {
        server_name: server.display_name,
        server_type: server.server_type.raw_value().to_string(),
        min_ram_gb: server.min_ram_gb,
        max_ram_gb: server.max_ram_gb,
        physical_ram_gb: physical,
        recommended_max_gb: recommended_max_ram_gb(physical),
        server_running: state.status_snapshot().running,
        has_active_server: true,
    })
    .into_response()
}

pub async fn set_ram_config(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<axum::Json<RamConfigUpdateRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let axum::Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if body.min_ram_gb.is_none() && body.max_ram_gb.is_none() {
        return invalid_body(
            "no_changes",
            "At least one of minRamGB/maxRamGB is required.",
        );
    }
    let Some(active_id) = state.active_server_id() else {
        return error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "No active server.",
        );
    };

    let result = state.try_mutate_config(|config| {
        let server = config
            .servers
            .iter_mut()
            .find(|s| s.id == active_id)
            .ok_or(())?;
        if let Some(min) = body.min_ram_gb {
            server.min_ram_gb = min;
        }
        if let Some(max) = body.max_ram_gb {
            server.max_ram_gb = max;
        }
        Ok::<(f64, f64), ()>((server.min_ram_gb, server.max_ram_gb))
    });

    match result {
        Ok((min_ram_gb, max_ram_gb)) => axum::Json(RamConfigUpdateResultDto {
            success: true,
            min_ram_gb: Some(min_ram_gb),
            max_ram_gb: Some(max_ram_gb),
            restart_required: state.status_snapshot().running,
            message: Some("RAM allocation updated.".to_string()),
        })
        .into_response(),
        Err(TryMutateError::Domain(())) => error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "No active server.",
        ),
        Err(TryMutateError::Save(error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

// ---------- POST /v1/java-runtimes/install ----------

pub async fn install_java_runtime(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<axum::Json<JavaRuntimeInstallRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let axum::Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if !MINECRAFT_INSTALL_OPTIONS
        .iter()
        .any(|option| option.major == body.major)
    {
        return invalid_body("invalid_major", "major must be one of 8, 17, 21, 25.");
    }
    let major = body.major as u32;
    let target = format!("java-runtime-{major}");

    let operation_id = match state.operations().begin_lifecycle(
        "java-download",
        Some(target),
        "Installing Java runtime.",
    ) {
        Ok(id) => id,
        Err(msc_application::operations::LifecycleOperationError::Conflict(error)) => {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "download_in_progress",
                &error.message,
            );
        }
        Err(error) => return operation_error_response(error),
    };

    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_install_java_runtime(worker_state, worker_operation_id, major)
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    axum::Json(JavaRuntimeInstallResultDto {
        success: true,
        message: "Java runtime install started.".to_string(),
        operation_id: operation_id.as_str().to_string(),
    })
    .into_response()
}

fn run_install_java_runtime(
    state: LifecycleRoutesState,
    operation_id: msc_domain::operation::OperationId,
    major: u32,
) {
    let transport = HttpTransport::new();
    let host = current_host_os();
    let arch = current_arch();
    let asset: Result<AdoptiumAsset, JavaRuntimeInstallError> =
        java_runtime_install::query_adoptium_latest(&transport, major, host, arch);
    let asset = match asset {
        Ok(asset) => asset,
        Err(error) => {
            let _ =
                state.finish_operation_failure(&operation_id, "internal_error", error.to_string());
            return;
        }
    };
    let runtime_name = format!("temurin-{major}-{}-{arch}", host.adoptium_os_param());
    match java_runtime_install::install_managed_runtime(
        &StdFileSystem,
        &transport,
        &runtimes_root(),
        &runtime_name,
        &asset,
    ) {
        Ok(path) => {
            let mut result = std::collections::BTreeMap::new();
            result.insert(
                "runtimePath".to_string(),
                path.to_string_lossy().into_owned(),
            );
            result.insert("major".to_string(), major.to_string());
            let _ = state.finish_operation_success(
                &operation_id,
                &format!("Installed Java {major}."),
                result,
            );
        }
        Err(error) => {
            let _ =
                state.finish_operation_failure(&operation_id, "internal_error", error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use axum::extract::{Query, State};

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

    #[tokio::test]
    async fn versions_route_reports_no_active_server() {
        let state = route_state();
        let response = versions(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response_body(response)
                .await
                .contains("\"supportsVersions\":false")
        );
    }

    #[tokio::test]
    async fn versions_for_create_defaults_bedrock_to_not_implemented() {
        let query = VersionsCreateQuery {
            server_type: Some("bedrock".to_string()),
            java_flavor: None,
        };
        let response = versions_for_create(Query(query)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert!(body.contains("\"isBedrock\":true"));
        assert!(body.contains("\"supportsVersions\":false"));
    }

    #[tokio::test]
    async fn change_version_route_rejects_a_blank_version_id() {
        let state = route_state();
        let response = change_version(
            State(state),
            Extension(admin_credential()),
            Ok(axum::Json(VersionChangeRequestDto {
                version_id: "  ".to_string(),
                loader_version: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response).await.contains("missing_version_id"));
    }

    #[tokio::test]
    async fn change_version_route_reports_no_active_server() {
        let state = route_state();
        let response = change_version(
            State(state),
            Extension(admin_credential()),
            Ok(axum::Json(VersionChangeRequestDto {
                version_id: server_versions::LATEST.to_string(),
                loader_version: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response_body(response).await.contains("no_active_server"));
    }

    #[tokio::test]
    async fn set_ram_config_route_rejects_no_changes() {
        let state = route_state();
        let response = set_ram_config(
            State(state),
            Extension(admin_credential()),
            Ok(axum::Json(RamConfigUpdateRequestDto {
                min_ram_gb: None,
                max_ram_gb: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response).await.contains("no_changes"));
    }

    #[tokio::test]
    async fn set_ram_config_route_reports_no_active_server() {
        let state = route_state();
        let response = set_ram_config(
            State(state),
            Extension(admin_credential()),
            Ok(axum::Json(RamConfigUpdateRequestDto {
                min_ram_gb: Some(2.0),
                max_ram_gb: Some(4.0),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response_body(response).await.contains("no_active_server"));
    }

    #[tokio::test]
    async fn install_java_runtime_route_rejects_an_unrecognized_major() {
        let state = route_state();
        let response = install_java_runtime(
            State(state),
            Extension(admin_credential()),
            Ok(axum::Json(JavaRuntimeInstallRequestDto { major: 99 })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response).await.contains("invalid_major"));
    }

    #[tokio::test]
    async fn get_and_set_java_config_route_round_trips() {
        let state = route_state();
        let get_response = get_java_config(State(state.clone())).await;
        assert_eq!(get_response.status(), StatusCode::OK);

        let set_response = set_java_config(
            State(state.clone()),
            Extension(admin_credential()),
            Ok(axum::Json(JavaConfigSetRequestDto {
                executable_path: Some("/usr/bin/java".to_string()),
            })),
        )
        .await;
        assert_eq!(set_response.status(), StatusCode::OK);
        assert!(response_body(set_response).await.contains("/usr/bin/java"));

        let get_again = get_java_config(State(state)).await;
        assert!(response_body(get_again).await.contains("/usr/bin/java"));
    }

    // ---------------------------------------------------------------------
    // P7.31: `GET /v1/java-runtimes` now corroborates its path-inferred
    // `major_version` guess with a real probe -- proven against
    // `with_probed_major_version` directly, since `java_runtimes` itself
    // scans the real host filesystem/`$HOME` with no injectable roots.
    // ---------------------------------------------------------------------

    fn guessed_runtime(major_guess: i64) -> DetectedJavaRuntime {
        DetectedJavaRuntime {
            name: "guessed".to_string(),
            executable_path: "/opt/jdk/bin/java".to_string(),
            home_path: "/opt/jdk".to_string(),
            major_version: Some(major_guess),
        }
    }

    #[test]
    fn java_runtime_probed_major_version_overrides_path_inferred_guess() {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        let driver = std::thread::spawn(move || {
            loop {
                if let Some((pid, _)) = supervisor.spawned_requests().into_iter().next() {
                    let _ = supervisor
                        .emit_stdout(pid, b"openjdk version \"21.0.1\" 2023-10-17\n".to_vec());
                    let _ = supervisor.exit_normally(pid);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        // Directory name suggested 17; the real probe says 21.
        let updated = with_probed_major_version(supervisor, guessed_runtime(17));
        driver.join().unwrap();

        assert_eq!(updated.major_version, Some(21));
    }

    #[test]
    fn java_runtime_probed_major_version_falls_back_to_guess_when_probe_fails() {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        supervisor.fail_next_spawn("no such executable");

        let updated = with_probed_major_version(supervisor, guessed_runtime(17));

        assert_eq!(updated.major_version, Some(17));
    }
}
