//! `GET /v1/capabilities` — `capability-model.md` §3's response.
//! `agentVersion`/`apiMajor`/`apiMinor`/`hostOs` are real compile-time
//! constants (P2.6's own distinction). Bedrock is reported from the runtime
//! selected during production composition, not from a route-local guess.

use axum::extract::Query;
use axum::{Extension, Json};
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, CapabilitiesResponseDto, HelpersDto, HostOsDto,
    JavaRuntimeCapabilityDto, MeResponseDto, ServerTypesDto, ThirdPartyWorldConfigBoundaryDto,
    WorldSettingCapabilityDto, WorldSettingsCapabilitiesDto, WorldSettingsContextDto,
};

use crate::auth::{AuthenticatedCredential, CredentialRole, role_to_string};
use crate::routes::networking::NetworkingState;

/// v1's fixed major/minor, per `versioning-and-errors.md` §2.
const API_MAJOR: u32 = 1;
const API_MINOR: u32 = 0;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesQuery {
    pub server_type: Option<String>,
    pub minecraft_version: Option<String>,
    pub java_flavor: Option<String>,
    pub loader_version: Option<String>,
    /// Used by diagnostics and the create wizard when a host has selected a
    /// non-default Java executable. It is never treated as a server config
    /// mutation; the route only probes it.
    pub java_runtime_path: Option<String>,
}

pub async fn capabilities(
    Query(query): Query<CapabilitiesQuery>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(networking): Extension<NetworkingState>,
) -> Json<CapabilitiesResponseDto> {
    capabilities_for_query(credential, networking, Query(query)).await
}

async fn capabilities_for_query(
    credential: AuthenticatedCredential,
    networking: NetworkingState,
    Query(query): Query<CapabilitiesQuery>,
) -> Json<CapabilitiesResponseDto> {
    let config = networking.lifecycle.app_config_snapshot();
    let active = config
        .active_server_id
        .as_ref()
        .and_then(|id| config.servers.iter().find(|server| &server.id == id));
    let selected_server_type = query
        .server_type
        .as_deref()
        .and_then(msc_domain::identity::ServerType::from_raw_value)
        .or_else(|| active.map(|server| server.server_type));
    let selected_version_raw = query
        .minecraft_version
        .filter(|value| !value.trim().is_empty())
        .or_else(|| active.and_then(|server| server.minecraft_version.clone()));
    let selected_flavor = query
        .java_flavor
        .as_deref()
        .and_then(msc_domain::identity::JavaServerFlavor::from_raw_value)
        .or_else(|| active.map(|server| server.java_flavor));
    let selected_version = crate::routes::versions::minecraft_version_from_selection(
        selected_flavor
            .filter(|_| selected_server_type == Some(msc_domain::identity::ServerType::Java)),
        selected_version_raw,
    );
    let selected_loader = query
        .loader_version
        .filter(|value| !value.trim().is_empty())
        .or_else(|| active.and_then(|server| server.loader_version.clone()));
    let java_runtime_path = query
        .java_runtime_path
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.java_path.clone());
    let bedrock_runtime = networking.lifecycle.bedrock_runtime_state();
    let bedrock_supported = bedrock_runtime.state == "available";
    let bedrock_backend = bedrock_runtime.backend;
    let java_runtime = match selected_server_type {
        Some(msc_domain::identity::ServerType::Java) => Some(
            inspect_java_runtime(
                networking.lifecycle.process_supervisor(),
                java_runtime_path,
                selected_version.clone(),
            )
            .await,
        ),
        _ => None,
    };
    let world_settings = selected_server_type.map(|server_type| {
        world_settings_capabilities(
            msc_domain::capability::WorldCapabilityContext {
                server_type,
                minecraft_version: selected_version,
                java_flavor: selected_flavor
                    .filter(|_| server_type == msc_domain::identity::ServerType::Java),
                loader_version: selected_loader,
            },
            java_runtime,
        )
    });

    Json(CapabilitiesResponseDto {
        base: CapabilitiesDto {
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            api_major: API_MAJOR,
            api_minor: API_MINOR,
            host_os: host_os(),
            permissions: credential.permissions,
            server_types: ServerTypesDto {
                vanilla: true,
                paper: true,
                fabric: true,
                forge: true,
                neoforge: true,
                bedrock: BedrockSupportDto {
                    supported: bedrock_supported,
                    backend: bedrock_backend,
                    runtime: Some(bedrock_runtime),
                },
            },
            helpers: HelpersDto {
                playit: active.is_some_and(|server| server.playit_enabled),
                duckdns: config.duckdns_hostname.is_some(),
                geyser: active.is_some_and(|server| {
                    msc_application::geyser::installation(
                        &msc_infrastructure::fs::StdFileSystem,
                        std::path::Path::new(&server.server_dir),
                    )
                    .geyser_installed
                }),
                tailscale: Some(tailscale_is_installed()),
            },
        },
        world_settings,
    })
}

async fn inspect_java_runtime(
    supervisor: &'static (dyn msc_infrastructure::process::ProcessSupervisor + Send + Sync),
    executable_path: String,
    minecraft_version: Option<String>,
) -> JavaRuntimeCapabilityDto {
    let path_for_probe = executable_path.clone();
    let probe = tokio::task::spawn_blocking(move || {
        msc_infrastructure::java_runtime_detection::run_java_version_probe(
            supervisor,
            &path_for_probe,
        )
    })
    .await
    .unwrap_or(msc_domain::java_runtime::JavaVersionProbe::NotFound);
    let required_major =
        msc_domain::java_runtime::required_java_major(minecraft_version.as_deref());
    match probe {
        msc_domain::java_runtime::JavaVersionProbe::NotFound => JavaRuntimeCapabilityDto {
            state: "unavailable".to_string(),
            executable_path: Some(executable_path),
            required_major: Some(required_major),
            detected_major: None,
            reason: Some("No usable Java runtime was found at the configured path.".to_string()),
        },
        msc_domain::java_runtime::JavaVersionProbe::Captured { output } => {
            if let Err(error) =
                msc_domain::java_runtime::validate_looks_like_java(&executable_path, &output)
            {
                return JavaRuntimeCapabilityDto {
                    state: "unavailable".to_string(),
                    executable_path: Some(executable_path),
                    required_major: Some(required_major),
                    detected_major: None,
                    reason: Some(error.to_string()),
                };
            }
            let detected_major = msc_domain::java_runtime::parse_major(&output);
            let (state, reason) = match detected_major {
                Some(major) if major < required_major => (
                    "unavailable",
                    Some(format!(
                        "Minecraft {} needs Java {required_major}, but the configured runtime is Java {major}.",
                        minecraft_version.as_deref().unwrap_or("this version")
                    )),
                ),
                Some(_) => ("available", None),
                None => (
                    "unknown",
                    Some(
                        "The Java runtime responded, but its version could not be read."
                            .to_string(),
                    ),
                ),
            };
            JavaRuntimeCapabilityDto {
                state: state.to_string(),
                executable_path: Some(executable_path),
                required_major: Some(required_major),
                detected_major,
                reason,
            }
        }
    }
}

fn world_settings_capabilities(
    context: msc_domain::capability::WorldCapabilityContext,
    java_runtime: Option<JavaRuntimeCapabilityDto>,
) -> WorldSettingsCapabilitiesDto {
    let native_capabilities = msc_domain::capability::native_world_capabilities(&context);
    let fields = msc_domain::capability::world_setting_capabilities(&context)
        .into_iter()
        .map(|field| {
            let available =
                field.state == msc_domain::capability::WorldSettingCapabilityState::Available;
            (
                field.field.key().to_string(),
                WorldSettingCapabilityDto {
                    capability: field.capability,
                    state: field.state.raw_value().to_string(),
                    available,
                    reason: field.reason,
                    help_id: field.help_id,
                },
            )
        })
        .collect();
    WorldSettingsCapabilitiesDto {
        context: WorldSettingsContextDto {
            server_type: context.server_type.raw_value().to_string(),
            minecraft_version: context.minecraft_version,
            java_flavor: context.java_flavor.map(|flavor| flavor.raw_value().to_string()),
            loader_version: context.loader_version,
            java_runtime,
            native_capabilities,
        },
        fields,
        third_party: ThirdPartyWorldConfigBoundaryDto {
            available: false,
            label: "Provided by this server/mod".to_string(),
            message: "MSC manages native world settings only. Use the server or mod's own configuration path for custom settings; MSC will not invent a universal editor or silently discard them.".to_string(),
            handoff: "server_settings".to_string(),
            help_id: Some("handbook.standard-vs-modded".to_string()),
        },
    }
}

fn tailscale_is_installed() -> bool {
    let candidates = [
        "tailscale",
        "/usr/local/bin/tailscale",
        "/opt/homebrew/bin/tailscale",
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        r"C:\Program Files\Tailscale\tailscale.exe",
        r"C:\Program Files (x86)\Tailscale\tailscale.exe",
    ];
    candidates.iter().any(|candidate| {
        std::process::Command::new(candidate)
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

/// `GET /v1/me` — the calling token's own role/name/permissions, read
/// straight off the `AuthenticatedCredential` the auth middleware already
/// attached to the request. No lookup of its own: every field here was
/// already computed once, at auth time.
pub async fn me(Extension(credential): Extension<AuthenticatedCredential>) -> Json<MeResponseDto> {
    Json(MeResponseDto {
        role: role_to_string(credential.role),
        name: credential.label,
        permissions: credential.permissions,
        is_named_token: matches!(credential.role, CredentialRole::Named),
    })
}

fn host_os() -> HostOsDto {
    match std::env::consts::OS {
        "macos" => HostOsDto::Macos,
        "linux" => HostOsDto::Linux,
        "windows" => HostOsDto::Windows,
        other => panic!("unsupported host OS: {other}"),
    }
}
