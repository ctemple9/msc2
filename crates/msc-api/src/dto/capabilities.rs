//! `CapabilitiesDTO` — `capability-model.md` §3's `GET /v1/capabilities`
//! response shape, confirmed by Cameron Temple 2026-07-31.
//!
//! Deliberately independent of `msc_domain::capability`, for the same
//! module-boundary reason `operation.rs` gives: the domain crate carries
//! no serde dependency, so the wire representation lives here.

use serde::{Deserialize, Serialize};

/// `msc2-engineering.md` §8's first support matrix ("MSC agent host").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostOsDto {
    Macos,
    Linux,
    Windows,
}

/// D-019's nine-category permission vocabulary (still **Proposed**,
/// pending Cameron's confirmation) — P2.1's validation against all 88
/// baseline routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionCategoryDto {
    ServerControl,
    Players,
    Settings,
    Addons,
    Worlds,
    Broadcast,
    Networking,
    Fleet,
    Admin,
}

/// `serverTypes.bedrock.backend`. `None` means "not supported on this
/// host."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BedrockBackendDto {
    Native,
    #[serde(rename = "vz-sidecar")]
    VzSidecar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BedrockSupportDto {
    pub supported: bool,
    /// Required-but-nullable on the wire (`capability-model.md` §3's
    /// `bedrock` object requires both keys present; `backend` is `null`
    /// when `supported` is `false`) — unlike this crate's other optional
    /// fields, this one is always serialized, never omitted.
    #[serde(default)]
    pub backend: Option<BedrockBackendDto>,
}

/// One boolean per Java flavor (`vanilla`, `paper`, `fabric`, `forge`,
/// `neoforge`) plus `bedrock` — capability-model.md §3's exact field list,
/// deliberately narrower than `identity::JavaServerFlavor`'s full
/// nine-case set. **Placeholder this phase**: real per-flavor detection is
/// Phase 4/10 work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerTypesDto {
    pub vanilla: bool,
    pub paper: bool,
    pub fabric: bool,
    pub forge: bool,
    pub neoforge: bool,
    pub bedrock: BedrockSupportDto,
}

/// Installed-helper presence flags. **Placeholder this phase**: real
/// presence detection is Phase 3 substrate work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpersDto {
    pub playit: bool,
    pub duckdns: bool,
    pub geyser: bool,
}

/// `GET /v1/me`'s response shape (`openapi.json`'s `MeResponseDTO`): the
/// calling token's own role/name/permissions, echoed back from the
/// `AuthenticatedCredential` the auth middleware already attached to the
/// request — no lookup of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponseDto {
    pub role: String,
    pub name: String,
    pub permissions: Vec<PermissionCategoryDto>,
    pub is_named_token: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesDto {
    /// The running agent build's own semver string. Real this phase — a
    /// compile-time constant, not a placeholder.
    pub agent_version: String,
    pub api_major: u32,
    pub api_minor: u32,
    /// Real this phase: `std::env::consts::OS` is a compile-time constant.
    pub host_os: HostOsDto,
    /// The calling token's granted categories. Real this phase: P2.3's
    /// fixed dev token has a fixed permission set, echoed back directly.
    pub permissions: Vec<PermissionCategoryDto>,
    pub server_types: ServerTypesDto,
    pub helpers: HelpersDto,
}
