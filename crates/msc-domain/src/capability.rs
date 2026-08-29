//! The capability domain type: a pure `CapabilitySet` describing what a
//! given agent build, on a given host, can do for a given token.
//!
//! Greenfield MSC 2 construction, not a port — MSC 1 has no equivalent
//! route or type. The specification is
//! `docs/msc2/api-contract/capability-model.md` (P2.6, confirmed by
//! Cameron Temple 2026-07-31) and `docs/msc2/msc2-decisions.md`'s D-019
//! (the nine-category permission vocabulary, P2.1). This module implements
//! their data shape only.
//!
//! No I/O: per `msc2-engineering.md` §6's module-boundary rule, the real
//! detection logic that *populates* a `CapabilitySet` (installed-helper
//! probing, per-flavor Java support checks, Bedrock backend selection) is
//! Phase 3/4/10 infrastructure work, not this crate's job. Wire
//! serialization (the JSON `CapabilitiesDTO` shape) is `msc-api`'s job —
//! this crate takes on no serde dependency.

use crate::identity::{JavaServerFlavor, ServerType};
use crate::world_profile::WorldProfileField;
use std::collections::BTreeSet;

/// closed enum, `msc2-engineering.md` §8's first support matrix ("MSC
/// agent host") — capability-model.md §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HostOs {
    Macos,
    Linux,
    Windows,
}

impl HostOs {
    pub const ALL: [Self; 3] = [Self::Macos, Self::Linux, Self::Windows];

    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "macos" => Some(Self::Macos),
            "linux" => Some(Self::Linux),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }
}

/// The nine-category permission vocabulary D-019 validated against all 88
/// baseline routes (P2.1) — still **Proposed**, pending Cameron's
/// confirmation, per D-019's own status. `Admin` formalizes what MSC 1
/// left as an implicit "absent from the permission map" gate (D-019
/// finding 2); the other eight carry MSC 1's own enforced category
/// strings forward, with `mods` renamed `Addons` to match the code
/// (finding 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PermissionCategory {
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

impl PermissionCategory {
    /// D-019's revised-decision declaration order.
    pub const ALL: [Self; 9] = [
        Self::ServerControl,
        Self::Players,
        Self::Settings,
        Self::Addons,
        Self::Worlds,
        Self::Broadcast,
        Self::Networking,
        Self::Fleet,
        Self::Admin,
    ];

    pub fn raw_value(self) -> &'static str {
        match self {
            Self::ServerControl => "serverControl",
            Self::Players => "players",
            Self::Settings => "settings",
            Self::Addons => "addons",
            Self::Worlds => "worlds",
            Self::Broadcast => "broadcast",
            Self::Networking => "networking",
            Self::Fleet => "fleet",
            Self::Admin => "admin",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "serverControl" => Some(Self::ServerControl),
            "players" => Some(Self::Players),
            "settings" => Some(Self::Settings),
            "addons" => Some(Self::Addons),
            "worlds" => Some(Self::Worlds),
            "broadcast" => Some(Self::Broadcast),
            "networking" => Some(Self::Networking),
            "fleet" => Some(Self::Fleet),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

/// `serverTypes.bedrock.backend` — capability-model.md §3, designed
/// against §9's native-vs-VZ-sidecar story. `None` on the DTO means "not
/// supported on this host," carried here as the absence of a
/// `BedrockBackend` on an unsupported `BedrockSupport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockBackend {
    Native,
    VzSidecar,
}

impl BedrockBackend {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::VzSidecar => "vz-sidecar",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "native" => Some(Self::Native),
            "vz-sidecar" => Some(Self::VzSidecar),
            _ => None,
        }
    }
}

/// `serverTypes.bedrock` — capability-model.md §3: `supported` bool plus
/// the backend, `None` when unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BedrockSupport {
    pub supported: bool,
    pub backend: Option<BedrockBackend>,
}

/// `serverTypes` — capability-model.md §3 and §3's own field-by-field
/// note: "one boolean per Java flavor (`vanilla`, `paper`, `fabric`,
/// `forge`, `neoforge`)". Deliberately not `identity::JavaServerFlavor`'s
/// full nine-case set — the confirmed contract names exactly these five,
/// omitting `purpur`/`pufferfish`/`spigot`/`quilt`, so this type mirrors
/// the contract as confirmed rather than the broader flavor enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerTypeSupport {
    pub vanilla: bool,
    pub paper: bool,
    pub fabric: bool,
    pub forge: bool,
    pub neoforge: bool,
    pub bedrock: BedrockSupport,
}

/// `helpers` — capability-model.md §3: installed-helper presence flags
/// for the three genuine external helper integrations (Playit.gg, Geyser,
/// DuckDNS); Xbox Broadcast is LAN-native and has no installed-helper flag
/// of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelperPresence {
    pub playit: bool,
    pub duckdns: bool,
    pub geyser: bool,
}

/// `CapabilitiesDTO`'s full data shape — capability-model.md §3.
/// Deliberately excludes per-server state (§7: confirmed out of scope —
/// this type is host-and-token-scoped only, not per-server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySet {
    pub agent_version: String,
    pub api_major: u32,
    pub api_minor: u32,
    pub host_os: HostOs,
    pub permissions: BTreeSet<PermissionCategory>,
    pub server_types: ServerTypeSupport,
    pub helpers: HelperPresence,
}

/// The selected server context used to evaluate world-setting support. This
/// is separate from [`CapabilitySet`] because the latter remains the small
/// host-and-token capability envelope used by older clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCapabilityContext {
    pub server_type: ServerType,
    pub minecraft_version: Option<String>,
    pub java_flavor: Option<JavaServerFlavor>,
    pub loader_version: Option<String>,
}

/// Why a world setting can or cannot be presented as an applicable setting.
/// `Unknown` is intentionally distinct from `Unsupported`: an unselected
/// version must not turn into a false promise about a future server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSettingCapabilityState {
    Available,
    Unsupported,
    Unknown,
}

impl WorldSettingCapabilityState {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

/// One advertised world-profile field. The capability layer returns every
/// known field, including unavailable ones, so clients can explain why a
/// control is absent without guessing from a Java/Bedrock static list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSettingCapability {
    pub field: WorldProfileField,
    pub capability: String,
    pub state: WorldSettingCapabilityState,
    pub reason: Option<String>,
    pub help_id: Option<String>,
}

/// MSC 2's native world-settings floor for this release. The runtime's
/// ability to launch a server is reported beside these fields; it is not
/// silently conflated with whether a persisted native setting has a known
/// Minecraft meaning.
pub const NATIVE_WORLD_SETTINGS_MIN_VERSION: &str = "1.20";

/// Evaluate native world settings for one concrete server selection.
///
/// The common profile is deliberately small and version-bounded. Java flavor
/// and loader are retained in the capability name so a client can show the
/// actual context, while unknown or unsupported selections remain explicit.
pub fn world_setting_capabilities(context: &WorldCapabilityContext) -> Vec<WorldSettingCapability> {
    WorldProfileField::ALL
        .into_iter()
        .map(|field| {
            let (state, reason) = if !field.applies_to(context.server_type) {
                (
                    WorldSettingCapabilityState::Unsupported,
                    Some(match context.server_type {
                        ServerType::Java => "This setting belongs to Bedrock servers.",
                        ServerType::Bedrock => "This setting belongs to Java servers.",
                    }),
                )
            } else if field != WorldProfileField::SafetyState && context.minecraft_version.is_none()
            {
                (
                    WorldSettingCapabilityState::Unknown,
                    Some("Minecraft version has not been selected."),
                )
            } else if field != WorldProfileField::SafetyState
                && !minecraft_version_at_least_1_20(
                    context.minecraft_version.as_deref().unwrap_or_default(),
                )
            {
                (
                    WorldSettingCapabilityState::Unsupported,
                    Some("Requires Minecraft 1.20 or newer."),
                )
            } else if context.server_type == ServerType::Java
                && field.capability() == "world.java"
                && context.java_flavor.is_none()
            {
                (
                    WorldSettingCapabilityState::Unknown,
                    Some("Java server flavor has not been selected."),
                )
            } else {
                (WorldSettingCapabilityState::Available, None)
            };

            WorldSettingCapability {
                field,
                capability: capability_name(field, context),
                state,
                reason: reason.map(str::to_string),
                help_id: field.help_id().map(str::to_string),
            }
        })
        .collect()
}

/// Names the native server family in addition to the broad Java/Bedrock
/// bucket. The names are data, not executable configuration paths; arbitrary
/// mod-defined settings stay outside this list and use the explicit handoff
/// advertised by the API/UI.
pub fn native_world_capabilities(context: &WorldCapabilityContext) -> Vec<String> {
    match context.server_type {
        ServerType::Bedrock => vec!["world.bedrock".to_string()],
        ServerType::Java => {
            let mut capabilities = vec!["world.java".to_string()];
            if let Some(flavor) = context.java_flavor {
                capabilities.push(format!("world.java.{}", flavor.raw_value()));
                capabilities.push(format!(
                    "world.java.{}.{}",
                    flavor.raw_value(),
                    match flavor {
                        JavaServerFlavor::Paper
                        | JavaServerFlavor::Purpur
                        | JavaServerFlavor::Pufferfish
                        | JavaServerFlavor::Vanilla
                        | JavaServerFlavor::Spigot => "standard",
                        JavaServerFlavor::Fabric
                        | JavaServerFlavor::NeoForge
                        | JavaServerFlavor::Forge
                        | JavaServerFlavor::Quilt => "modded",
                    }
                ));
            }
            if let Some(loader) = &context.loader_version {
                capabilities.push(format!("loader.{}", loader));
            }
            capabilities
        }
    }
}

fn capability_name(field: WorldProfileField, context: &WorldCapabilityContext) -> String {
    match (context.server_type, context.java_flavor, field.capability()) {
        (ServerType::Java, Some(flavor), "world.java") => {
            format!("world.java.{}", flavor.raw_value())
        }
        _ => field.capability().to_string(),
    }
}

fn minecraft_version_at_least_1_20(version: &str) -> bool {
    let numbers: Vec<u32> = version
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    match numbers.as_slice() {
        [1, minor, ..] => *minor >= 20,
        [major, ..] => *major > 1,
        [] => false,
    }
}
