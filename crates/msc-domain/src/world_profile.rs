//! The world-local half of a server's configuration.
//!
//! MSC 1 presents world values and server values together because both happen
//! to be edited from one screen. That is not an ownership boundary. A world
//! profile travels with a `WorldSlot`; the active runtime is a projection of
//! that profile. Server-wide policy (ports, player limits, access, process,
//! and helper settings) stays in the server profile.
//!
//! This module deliberately contains no filesystem or Minecraft-version
//! probing. It defines the stable vocabulary and change policy that the
//! persistence and capability layers populate in later steps.

use crate::identity::ServerType;
use std::collections::BTreeMap;

/// Version written beside every persisted world profile.
pub const WORLD_PROFILE_SCHEMA_VERSION: u32 = 1;

/// Who owns a setting's source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingOwner {
    ServerProfile,
    WorldProfile,
}

impl SettingOwner {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::ServerProfile => "server_profile",
            Self::WorldProfile => "world_profile",
        }
    }
}

/// How a value may be changed without pretending a write took effect sooner
/// than the Minecraft runtime can actually use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingApplyPolicy {
    /// Minecraft consumes this value while generating a new world and does
    /// not safely reinterpret it for an already-generated one.
    CreationOnly,
    /// The value is read when a slot becomes active. It is not a second live
    /// server setting and is not silently copied into another slot.
    ApplyOnActivation,
    /// The active runtime can accept the change without a restart.
    LiveSafe,
    /// The value is persisted now but requires the server to restart before
    /// the runtime can truthfully report it as applied.
    RestartRequired,
}

impl SettingApplyPolicy {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::CreationOnly => "creation_only",
            Self::ApplyOnActivation => "apply_on_activation",
            Self::LiveSafe => "live_safe",
            Self::RestartRequired => "restart_required",
        }
    }
}

/// State of an individual value when an existing world is inspected.
///
/// `Configured` is used for an MSC-owned value. `Detected` means the value was
/// read from world data rather than assumed. The other states are deliberately
/// explicit so an older runtime or a Bedrock achievement restriction cannot
/// be rendered as a confident value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldValueState {
    Configured,
    Detected,
    Unknown,
    Unsupported,
    AchievementDisabled,
}

impl WorldValueState {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Detected => "detected",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::AchievementDisabled => "achievement_disabled",
        }
    }
}

/// Overall safety disclosure for a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSafetyState {
    Safe,
    AchievementDisabled,
    Unknown,
    Unsupported,
}

impl WorldSafetyState {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::AchievementDisabled => "achievement_disabled",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
        }
    }
}

/// The fields that can belong to a world profile. The stable keys are also
/// the keys used by the API's `fieldMetadata` object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldProfileField {
    IdentityName,
    IdentityLevelName,
    IdentitySeed,
    GenerationWorldType,
    GenerationFlatPreset,
    GenerationStructures,
    GenerationBiomeSource,
    GenerationGeneratorOptions,
    GenerationBonusChest,
    GenerationDataPacks,
    GameplayDifficulty,
    GameplayDefaultGameMode,
    GameplayHardcore,
    GameplayCommands,
    GameplayGamerules,
    GameplayCheats,
    GameplayExperiments,
    GameplayCoordinates,
    GameplayStartingMap,
    GameplaySupportedToggles,
    SafetyState,
}

impl WorldProfileField {
    /// Stable declaration order for metadata generation and documentation.
    pub const ALL: [Self; 21] = [
        Self::IdentityName,
        Self::IdentityLevelName,
        Self::IdentitySeed,
        Self::GenerationWorldType,
        Self::GenerationFlatPreset,
        Self::GenerationStructures,
        Self::GenerationBiomeSource,
        Self::GenerationGeneratorOptions,
        Self::GenerationBonusChest,
        Self::GenerationDataPacks,
        Self::GameplayDifficulty,
        Self::GameplayDefaultGameMode,
        Self::GameplayHardcore,
        Self::GameplayCommands,
        Self::GameplayGamerules,
        Self::GameplayCheats,
        Self::GameplayExperiments,
        Self::GameplayCoordinates,
        Self::GameplayStartingMap,
        Self::GameplaySupportedToggles,
        Self::SafetyState,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::IdentityName => "identity.name",
            Self::IdentityLevelName => "identity.level-name",
            Self::IdentitySeed => "identity.seed",
            Self::GenerationWorldType => "generation.world-type",
            Self::GenerationFlatPreset => "generation.flat-preset",
            Self::GenerationStructures => "generation.structures",
            Self::GenerationBiomeSource => "generation.biome-source",
            Self::GenerationGeneratorOptions => "generation.generator-options",
            Self::GenerationBonusChest => "generation.bonus-chest",
            Self::GenerationDataPacks => "generation.data-packs",
            Self::GameplayDifficulty => "gameplay.difficulty",
            Self::GameplayDefaultGameMode => "gameplay.default-game-mode",
            Self::GameplayHardcore => "gameplay.hardcore",
            Self::GameplayCommands => "gameplay.commands",
            Self::GameplayGamerules => "gameplay.gamerules",
            Self::GameplayCheats => "gameplay.cheats",
            Self::GameplayExperiments => "gameplay.experiments",
            Self::GameplayCoordinates => "gameplay.coordinates",
            Self::GameplayStartingMap => "gameplay.starting-map",
            Self::GameplaySupportedToggles => "gameplay.supported-toggles",
            Self::SafetyState => "safety.state",
        }
    }

    pub const fn capability(self) -> &'static str {
        match self {
            Self::IdentityName | Self::IdentityLevelName | Self::IdentitySeed => "world.identity",
            Self::GenerationFlatPreset
            | Self::GenerationBiomeSource
            | Self::GenerationGeneratorOptions
            | Self::GenerationDataPacks
            | Self::GameplayHardcore
            | Self::GameplayCommands => "world.java",
            Self::GenerationWorldType
            | Self::GenerationStructures
            | Self::GenerationBonusChest
            | Self::GameplayDifficulty
            | Self::GameplayDefaultGameMode
            | Self::GameplayGamerules => "world.common",
            Self::GameplayCheats
            | Self::GameplayExperiments
            | Self::GameplayCoordinates
            | Self::GameplayStartingMap
            | Self::GameplaySupportedToggles => "world.bedrock",
            Self::SafetyState => "world.safety",
        }
    }

    pub const fn apply_policy(self) -> SettingApplyPolicy {
        match self {
            Self::IdentityName | Self::IdentityLevelName => SettingApplyPolicy::ApplyOnActivation,
            Self::IdentitySeed
            | Self::GenerationWorldType
            | Self::GenerationFlatPreset
            | Self::GenerationStructures
            | Self::GenerationBiomeSource
            | Self::GenerationGeneratorOptions
            | Self::GenerationBonusChest
            | Self::GameplayHardcore
            | Self::GameplayCommands
            | Self::GameplayStartingMap => SettingApplyPolicy::CreationOnly,
            Self::GenerationDataPacks => SettingApplyPolicy::ApplyOnActivation,
            Self::GameplayDifficulty
            | Self::GameplayDefaultGameMode
            | Self::GameplayGamerules
            | Self::GameplayCoordinates
            | Self::GameplaySupportedToggles => SettingApplyPolicy::LiveSafe,
            Self::GameplayCheats | Self::GameplayExperiments => SettingApplyPolicy::RestartRequired,
            Self::SafetyState => SettingApplyPolicy::ApplyOnActivation,
        }
    }

    pub const fn help_id(self) -> Option<&'static str> {
        match self {
            Self::GameplayDifficulty => Some("settings.difficulty"),
            Self::IdentityName
            | Self::IdentityLevelName
            | Self::IdentitySeed
            | Self::GenerationWorldType
            | Self::GenerationFlatPreset
            | Self::GenerationStructures
            | Self::GenerationBiomeSource
            | Self::GenerationGeneratorOptions
            | Self::GenerationBonusChest
            | Self::GenerationDataPacks => Some("handbook.worlds-backups"),
            Self::GameplayDefaultGameMode => Some("settings.gamemode"),
            Self::GameplayCheats
            | Self::GameplayExperiments
            | Self::GameplayCoordinates
            | Self::GameplaySupportedToggles => Some("handbook.bedrock"),
            _ => Some("handbook.worlds-backups"),
        }
    }

    pub fn applies_to(self, server_type: ServerType) -> bool {
        match self {
            Self::GenerationFlatPreset
            | Self::GenerationBiomeSource
            | Self::GenerationGeneratorOptions
            | Self::GenerationDataPacks
            | Self::GameplayHardcore
            | Self::GameplayCommands => server_type == ServerType::Java,
            Self::GameplayCheats
            | Self::GameplayExperiments
            | Self::GameplayCoordinates
            | Self::GameplayStartingMap
            | Self::GameplaySupportedToggles => server_type == ServerType::Bedrock,
            _ => true,
        }
    }
}

/// A server-setting definition used by the settings-schema layer while the
/// active runtime still exposes legacy `server.properties` keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingContract {
    pub owner: SettingOwner,
    pub apply_policy: SettingApplyPolicy,
    pub capability: &'static str,
    pub help_id: Option<&'static str>,
}

/// Identity that travels with one world slot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldIdentity {
    pub name: Option<String>,
    pub level_name: Option<String>,
    pub seed: Option<String>,
}

/// Generation choices. `generator_options` is kept as an opaque value so a
/// newer Minecraft version can round-trip a shape this version does not know.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldGeneration {
    pub world_type: Option<String>,
    pub flat_preset: Option<String>,
    pub structures: Option<bool>,
    pub biome_source: Option<String>,
    pub generator_options: Option<String>,
    pub bonus_chest: Option<bool>,
    pub data_packs: Vec<String>,
}

/// Gameplay values stored with a world. Maps are intentional: gamerules,
/// experiments, and edition-specific toggles are open sets rather than a
/// client-owned enum that would silently discard a newer key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldGameplay {
    pub difficulty: Option<String>,
    pub default_game_mode: Option<String>,
    pub hardcore: Option<bool>,
    pub commands: Option<bool>,
    pub gamerules: BTreeMap<String, String>,
    pub cheats: Option<bool>,
    pub experiments: BTreeMap<String, bool>,
    pub coordinates: Option<bool>,
    pub starting_map: Option<bool>,
    pub supported_toggles: BTreeMap<String, bool>,
}

/// Safety is disclosed separately from gameplay values because Bedrock can
/// retain an achievement-disabled consequence after cheats or experiments are
/// later turned off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSafety {
    pub state: WorldSafetyState,
    pub reasons: Vec<String>,
}

impl Default for WorldSafety {
    fn default() -> Self {
        Self {
            state: WorldSafetyState::Unknown,
            reasons: Vec::new(),
        }
    }
}

/// A complete slot-local profile. Persistence, migration, and runtime
/// readback are deliberately owned by later application/infrastructure steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldProfile {
    pub schema_version: u32,
    pub identity: WorldIdentity,
    pub generation: WorldGeneration,
    pub gameplay: WorldGameplay,
    pub safety: WorldSafety,
}

impl Default for WorldProfile {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldProfile {
    pub fn new() -> Self {
        Self {
            schema_version: WORLD_PROFILE_SCHEMA_VERSION,
            identity: WorldIdentity::default(),
            generation: WorldGeneration::default(),
            gameplay: WorldGameplay::default(),
            safety: WorldSafety::default(),
        }
    }
}
