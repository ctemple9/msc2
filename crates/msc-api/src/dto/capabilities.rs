//! `CapabilitiesDTO` — `capability-model.md` §3's `GET /v1/capabilities`
//! response shape, confirmed by Cameron Temple 2026-07-31.
//!
//! Deliberately independent of `msc_domain::capability`, for the same
//! module-boundary reason `operation.rs` gives: the domain crate carries
//! no serde dependency, so the wire representation lives here.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ErrorDto;

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

/// Additive disclosure of the currently selected Bedrock runtime. The state
/// is intentionally a string so a newer agent can add a state without making
/// an older client unable to decode the rest of the response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockRuntimeStateDto {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BedrockBackendDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_os: Option<HostOsDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

impl BedrockRuntimeStateDto {
    /// Build the one public error envelope used when a Bedrock live
    /// operation cannot run on the selected host/backend.
    pub fn capability_unavailable_error(&self) -> ErrorDto {
        ErrorDto {
            code: "capability_unavailable".to_owned(),
            message: self
                .message
                .clone()
                .unwrap_or_else(|| "Bedrock runtime is unavailable.".to_owned()),
            help_id: self.help_id.clone(),
            details: Some(serde_json::json!({
                "capability": "bedrock-runtime",
                "serverType": "bedrock",
                "state": self.state,
                "backend": self.backend,
                "reasonCode": self.reason_code,
                "hostOs": self.host_os,
            })),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BedrockSupportDto {
    pub supported: bool,
    /// Required-but-nullable on the wire (`capability-model.md` §3's
    /// `bedrock` object requires both keys present; `backend` is `null`
    /// when `supported` is `false`) — unlike this crate's other optional
    /// fields, this one is always serialized, never omitted.
    #[serde(default)]
    pub backend: Option<BedrockBackendDto>,
    /// Authoritative current runtime state; `supported` and `backend` remain
    /// for older clients that do not know this additive field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

/// One boolean per Java flavor (`vanilla`, `paper`, `fabric`, `forge`,
/// `neoforge`) plus `bedrock` — capability-model.md §3's exact field list,
/// deliberately narrower than `identity::JavaServerFlavor`'s full
/// nine-case set. **Placeholder this phase**: real per-flavor detection is
/// Phase 4/10 work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Optional so a client can stay compatible with agents built before the
    /// first-launch Tailscale probe existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailscale: Option<bool>,
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

/// The runtime probe used while evaluating the selected server's native
/// world settings. A string state keeps older clients tolerant of future
/// probe outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeCapabilityDto {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_major: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_major: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The exact edition/flavor/version selection that produced a world-setting
/// capability response. This prevents a client from displaying a generic
/// "Java settings" label after the user changes the version or loader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSettingsContextDto {
    pub server_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_flavor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_runtime: Option<JavaRuntimeCapabilityDto>,
    #[serde(default)]
    pub native_capabilities: Vec<String>,
}

/// One field is always present in the map, even when it is unavailable. The
/// reason is part of the response so the UI can disable or hide it honestly
/// instead of reverse-engineering edition/version rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSettingCapabilityDto {
    pub capability: String,
    pub state: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

/// MSC-owned world settings stop at the native profile. Mod-defined settings
/// are deliberately handed back to the server/mod configuration path rather
/// than being guessed into a universal editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyWorldConfigBoundaryDto {
    pub available: bool,
    pub label: String,
    pub message: String,
    pub handoff: String,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSettingsCapabilitiesDto {
    pub context: WorldSettingsContextDto,
    pub fields: BTreeMap<String, WorldSettingCapabilityDto>,
    pub third_party: ThirdPartyWorldConfigBoundaryDto,
}

/// Additive extension of the original `CapabilitiesDTO`. Keeping the base
/// value as a nested field means existing typed Rust fixtures and older iOS
/// clients retain their original construction/decoding surface while the
/// wire response gains version-aware world settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilitiesResponseDto {
    #[serde(flatten)]
    pub base: CapabilitiesDto,
    #[serde(
        rename = "worldSettings",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub world_settings: Option<WorldSettingsCapabilitiesDto>,
}
