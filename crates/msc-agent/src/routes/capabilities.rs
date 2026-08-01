//! `GET /v1/capabilities` — `capability-model.md` §3's response.
//! `agentVersion`/`apiMajor`/`apiMinor`/`hostOs` are real compile-time
//! constants (P2.6's own distinction); everything else is placeholder
//! detection, standing in for Phase 3/4 substrate work.

use axum::Json;
use msc_api::dto::{
    BedrockSupportDto, CapabilitiesDto, HelpersDto, HostOsDto, PermissionCategoryDto,
    ServerTypesDto,
};

/// v1's fixed major/minor, per `versioning-and-errors.md` §2.
const API_MAJOR: u32 = 1;
const API_MINOR: u32 = 0;

pub async fn capabilities() -> Json<CapabilitiesDto> {
    Json(CapabilitiesDto {
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        api_major: API_MAJOR,
        api_minor: API_MINOR,
        host_os: host_os(),
        // The fixed dev token stands in for MSC 1's admin/named token
        // (`auth-scope-phase2.md` §2) — echoed back with every category,
        // matching the only role it plays this phase.
        permissions: vec![
            PermissionCategoryDto::ServerControl,
            PermissionCategoryDto::Players,
            PermissionCategoryDto::Settings,
            PermissionCategoryDto::Addons,
            PermissionCategoryDto::Worlds,
            PermissionCategoryDto::Broadcast,
            PermissionCategoryDto::Networking,
            PermissionCategoryDto::Fleet,
            PermissionCategoryDto::Admin,
        ],
        server_types: ServerTypesDto {
            vanilla: true,
            paper: true,
            fabric: true,
            forge: true,
            neoforge: true,
            bedrock: BedrockSupportDto {
                supported: false,
                backend: None,
            },
        },
        helpers: HelpersDto {
            playit: false,
            duckdns: false,
            geyser: false,
        },
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
