//! `GET /v1/capabilities` — `capability-model.md` §3's response.
//! `agentVersion`/`apiMajor`/`apiMinor`/`hostOs` are real compile-time
//! constants (P2.6's own distinction); everything else is placeholder
//! detection, standing in for Phase 3/4 substrate work.

use axum::{Extension, Json};
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, HelpersDto, HostOsDto, MeResponseDto, ServerTypesDto,
};

use crate::auth::{AuthenticatedCredential, CredentialRole, role_to_string};
use crate::routes::networking::NetworkingState;

/// v1's fixed major/minor, per `versioning-and-errors.md` §2.
const API_MAJOR: u32 = 1;
const API_MINOR: u32 = 0;

pub async fn capabilities(
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(networking): Extension<NetworkingState>,
) -> Json<CapabilitiesDto> {
    let config = networking.lifecycle.app_config_snapshot();
    let active = config
        .active_server_id
        .as_ref()
        .and_then(|id| config.servers.iter().find(|server| &server.id == id));
    Json(CapabilitiesDto {
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
                supported: false,
                backend: None,
                runtime: None,
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
        },
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
