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
