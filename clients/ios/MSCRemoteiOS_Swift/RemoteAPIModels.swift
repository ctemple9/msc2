import Foundation

// MARK: - Server Type

/// Mirrors the ServerType enum on the macOS side.
/// decodeIfPresent on the macOS API means older servers return nil here — default to .java.
enum ServerType: String, Codable, Hashable {
    case java
    case bedrock

    var displayName: String {
        switch self {
        case .java:    return "Java"
        case .bedrock: return "Bedrock"
        }
    }

    /// SF Symbol name for the badge shown in server pickers.
    var iconName: String {
        switch self {
        case .java:    return "cup.and.saucer.fill"
        case .bedrock: return "cube.fill"
        }
    }
}

enum RemoteJavaServerCategory: String, Codable, CaseIterable, Hashable {
    case standard
    case modded

    var displayName: String {
        switch self {
        case .standard: return "Standard"
        case .modded: return "Modded"
        }
    }
}

enum RemoteJavaServerFlavor: String, Codable, CaseIterable, Hashable {
    case paper
    case purpur
    case vanilla
    case fabric
    case neoforge
    case forge

    var displayName: String {
        switch self {
        case .paper: return "Paper"
        case .purpur: return "Purpur"
        case .vanilla: return "Vanilla"
        case .fabric: return "Fabric"
        case .neoforge: return "NeoForge"
        case .forge: return "Forge"
        }
    }

    var category: RemoteJavaServerCategory {
        switch self {
        case .paper, .purpur, .vanilla: return .standard
        case .fabric, .neoforge, .forge: return .modded
        }
    }

    var shortDescription: String {
        switch self {
        case .paper: return "Performance and plugin support."
        case .purpur: return "Paper plus extra gameplay settings."
        case .vanilla: return "Mojang's unmodified server."
        case .fabric: return "Lightweight mod loader."
        case .neoforge: return "Modern Forge-family mod loader."
        case .forge: return "Classic mod loader for modpacks."
        }
    }

    var iconName: String {
        switch self {
        case .paper: return "cup.and.saucer.fill"
        case .purpur: return "wand.and.stars"
        case .vanilla: return "cube"
        case .fabric: return "puzzlepiece.fill"
        case .neoforge: return "hammer.fill"
        case .forge: return "hammer.fill"
        }
    }

    var supportsCrossPlay: Bool {
        switch self {
        case .paper, .purpur: return true
        case .vanilla, .fabric, .neoforge, .forge: return false
        }
    }

    static func choices(in category: RemoteJavaServerCategory) -> [RemoteJavaServerFlavor] {
        allCases.filter { $0.category == category }
    }
}

// MARK: - Status & Servers

struct RemoteAPIStatus: Codable, Equatable {
    let running: Bool
    let activeServerId: String?
    let pid: Int?
    /// Nil when connecting to an older macOS app — treated as .java.
    let serverType: ServerType?
    /// Bedrock-only container hints. Nil for Java servers and older macOS builds.
    let dockerContainerRunning: Bool?
    let dockerContainerStatus: String?

    var resolvedServerType: ServerType { serverType ?? .java }
}

struct ServerDTO: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let directory: String
    /// Nil when connecting to an older macOS app that hasn't shipped E1 yet -- treated as .java.
    let serverType: ServerType?
    /// Java-only flavor. Nil on older macOS builds or for Bedrock servers -- treated as Paper for legacy Java servers.
    let javaFlavor: RemoteJavaServerFlavor?
    /// Game port (19132 for Bedrock, 25565 for Java). Nil on older macOS builds -- use type default.
    let gamePort: Int?
    /// Host address for the join card back (DuckDNS domain or public IP). Nil = not configured.
    let hostAddress: String?

    var resolvedServerType: ServerType { serverType ?? .java }
    var resolvedJavaFlavor: RemoteJavaServerFlavor { javaFlavor ?? .paper }
    var supportsJavaBedrockCrossPlay: Bool {
        resolvedServerType == .java && resolvedJavaFlavor.supportsCrossPlay
    }
    var supportsXboxBroadcastSettings: Bool {
        resolvedServerType == .bedrock || supportsJavaBedrockCrossPlay
    }

    /// The port to display on the join card. Falls back to protocol default if nil.
    var resolvedGamePort: Int {
        if let p = gamePort { return p }
        return resolvedServerType == .bedrock ? 19132 : 25565
    }

    /// Protocol label for join card.
    var protocolLabel: String { resolvedServerType == .bedrock ? "UDP" : "TCP" }
}

struct ServerRenameResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverId: String?
    let name: String?
}

struct ServerDeleteResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverId: String?
}

struct ServerCreateResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverId: String?
    let serverName: String?
    let warnings: [String]?
}

struct ServerEULAResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverId: String?
    let accepted: Bool?
}

struct ServerImportWorldDTO: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let sizeBytes: Int64
    let dimensionsLabel: String
}

struct ServerImportScanResponseDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let sourcePath: String?
    let isZip: Bool?
    let serverType: ServerType?
    let port: Int?
    let maxPlayers: Int?
    let eulaAccepted: Bool?
    let worlds: [ServerImportWorldDTO]?
    let defaultWorldName: String?
    let javaFlavor: String?
    let detectedMCVersion: String?
    let detectedLoaderVersion: String?
}

struct ServerImportResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverId: String?
    let serverName: String?
    let imported: Int?
    let skipped: Int?
    let replaced: Bool?
}

// MARK: - Console

struct ConsoleLineDTO: Identifiable, Equatable {
    /// Stable UUID assigned at decode time.
    ///
    /// Why not derive id from content? Minecraft servers routinely emit
    /// identical consecutive lines (join/leave messages, tick warnings,
    /// repeated status output). A content-derived id causes SwiftUI's
    /// ForEach to silently drop duplicate rows or produce animation glitches.
    /// A UUID generated once per received line is always unique.
    let id: UUID

    let ts: String
    let source: String
    let level: String?
    let text: String
}

// MARK: - Codable

extension ConsoleLineDTO: Codable {
    /// CodingKeys intentionally omits `id` — the server never sends it.
    private enum CodingKeys: String, CodingKey {
        case ts, source, level, text
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.ts     = try c.decode(String.self, forKey: .ts)
        self.source = try c.decode(String.self, forKey: .source)
        self.level  = try c.decodeIfPresent(String.self, forKey: .level)
        self.text   = try c.decode(String.self, forKey: .text)

        // UUID is generated here — once per decoded line, never from content.
        self.id = UUID()
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(ts,     forKey: .ts)
        try c.encode(source, forKey: .source)
        try c.encodeIfPresent(level, forKey: .level)
        try c.encode(text,   forKey: .text)
        // `id` is intentionally not encoded — it is client-side only.
    }
}

struct SimpleResult: Codable, Equatable {
    let result: String
    let activeServerId: String?
}

struct CommandResult: Codable, Equatable {
    let result: String
    let activeServerId: String?
    let command: String
}

// MARK: - Performance

/// Snapshot payload from the Remote API.
/// Keep fields optional so the client is robust if the server rolls out gradually.
struct PerformanceSnapshotDTO: Codable, Equatable {
    /// Timestamp string (recommended ISO8601). Optional for robustness.
    let ts: String?

    /// Paper TPS (1 minute). Typical range 0...20.
    /// Nil for Bedrock servers — BDS has no TPS concept.
    let tps1m: Double?

    /// Online players count.
    let playersOnline: Int?

    /// CPU usage percent (0...100).
    let cpuPercent: Double?

    /// RAM used (MB).
    let ramUsedMB: Double?

    /// RAM max/total (MB).
    let ramMaxMB: Double?

    /// World size (MB), if available.
    let worldSizeMB: Double?

    /// Server type of the currently active server.
    /// Nil when connecting to an older macOS app — treated as .java.
    let serverType: ServerType?

    var resolvedServerType: ServerType { serverType ?? .java }
}

// MARK: - Players

/// A single online player. UUID is optional — some server configs
/// don't expose it. Bedrock players have XUIDs, not Java UUIDs, so
/// the iOS client treats a nil UUID as "show generic icon".
struct PlayerDTO: Codable, Identifiable, Equatable {
    let name: String
    /// Java UUID (used for Crafatar avatar lookup). Nil for Bedrock players.
    let uuid: String?

    // Identifiable conformance — use name since UUIDs can be nil.
    // This is safe because two players with the same name cannot be
    // online simultaneously on a vanilla/Paper/BDS server.
    var id: String { name }
}

struct PlayersResponse: Codable, Equatable {
    let players: [PlayerDTO]
    let count: Int
    /// Optional context note from macOS. Currently used for Bedrock player-list caveats.
    let note: String?
}

// MARK: - Allowlist (Bedrock)

/// One Bedrock allowlist entry. `id` is the name (unique on the server).
struct AllowlistEntryDTO: Codable, Identifiable, Equatable {
    let name: String
    /// Xbox XUID, backfilled by the Mac when known. Nil = not yet resolved.
    let xuid: String?
    let ignoresPlayerLimit: Bool

    var id: String { name }
}

/// Response for GET /allowlist. `serverType` lets iOS decide whether to surface
/// the allowlist UI at all (Bedrock only). Older macOS builds that predate the
/// POST route still return this shape for GET.
struct AllowlistResponseDTO: Codable, Equatable {
    let serverType: String
    let entries: [AllowlistEntryDTO]

    var isBedrock: Bool { serverType == "bedrock" }
}

/// Response for POST /allowlist. Echoes the updated list so the client can
/// refresh in a single round-trip.
struct AllowlistMutationResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let serverType: String
    let entries: [AllowlistEntryDTO]
}

// MARK: - Components

struct ComponentStatusDTO: Codable, Identifiable, Equatable {
    let name: String
    let installedBuild: Int?
    let latestBuild: Int?
    let installedVersion: String?
    let latestVersion: String?
    let isUpToDate: Bool
    /// Server-formatted "installed" label for flavors without a Paper-style build
    /// number (Fabric, Vanilla, Forge, …). Nil from older servers or when not installed.
    let installedLabel: String?
    /// Whether this component uses the build-based Update flow. Nil (older servers) is
    /// treated as updatable only when a build number is present, preserving old behavior.
    let updatable: Bool?

    var id: String { name }

    /// True when the component is present on the server, regardless of whether it
    /// exposes a Paper-style build number.
    var isInstalled: Bool { installedBuild != nil || installedLabel != nil }

    /// Whether the in-app build-based Update button applies to this component.
    var isUpdatable: Bool { updatable ?? (installedBuild != nil) }

    /// Ready-to-display installed version line, e.g. "1.21.1 · build 130" or "1.21.1 · 0.16.5".
    var installedDisplay: String? {
        if let installedLabel { return installedLabel }
        if let installedBuild {
            let versionStr = installedVersion.map { "\($0) · " } ?? ""
            return "\(versionStr)build \(installedBuild)"
        }
        return nil
    }
}

struct ComponentsStatusDTO: Codable, Equatable {
    let components: [ComponentStatusDTO]
    let restartRequiredToApply: Bool
}

struct ComponentUpdateResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let newBuild: Int?
    let newVersion: String?
}

// MARK: - Add-ons (Modrinth-tracked mods / plugins)

struct AddonItemDTO: Codable, Identifiable, Equatable {
    let jarStem: String
    let displayName: String
    let isEnabled: Bool
    let projectId: String?
    let currentVersion: String?
    /// Non-nil when a newer compatible version exists on Modrinth.
    let availableVersion: String?
    /// "updateAvailable" | "upToDate" | "noCompatibleVersion" | "unlinked"
    let bucket: String
    let iconURL: String?

    var id: String { jarStem }
    var hasUpdate: Bool { bucket == "updateAvailable" }
}

struct AddonsResponseDTO: Codable, Equatable {
    let addons: [AddonItemDTO]
    let isResolving: Bool
    let serverSupportsAddons: Bool
    /// Nil when the server was not created from a .mrpack import (older server or non-pack server).
    let packManaged: Bool?
    /// The Modrinth modpack name, e.g. "Better MC [FORGE] BMC4". Nil for non-pack servers.
    let packName: String?

    var updateCount: Int { addons.filter { $0.hasUpdate }.count }
}

struct AddonUpdateResultDTO: Codable, Equatable {
    /// "update_started" | "no_updates_available" | "not_found" | "not_supported"
    let result: String
    let jarStem: String?
    let count: Int
}

struct AddonRemoveResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let jarStem: String
}

// MARK: - Catalog (add-on search + install)

struct CatalogItemDTO: Codable, Identifiable, Equatable {
    let projectId: String
    let slug: String
    let title: String
    let description: String
    let author: String
    let downloads: Int
    let iconURL: String?
    let isClientOnly: Bool
    let projectType: String

    var id: String { projectId }
}

struct CatalogSearchResponseDTO: Codable, Equatable {
    let supportsAddons: Bool
    let addonKind: String?
    let loaderName: String?
    let gameVersion: String?
    let results: [CatalogItemDTO]
    let note: String?
}

struct CatalogInstallResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let projectId: String
}

// MARK: - Settings (typed server.properties schema)
//
// The macOS server sends a self-describing schema (sections → typed fields);
// this screen renders a real form from it and never hardcodes the field set,
// so the same code serves Java, Bedrock (P4), and future config surfaces.

struct SettingOptionDTO: Codable, Equatable, Identifiable {
    let value: String
    let label: String
    var id: String { value }
}

struct SettingFieldDTO: Codable, Equatable, Identifiable {
    let key: String
    let label: String
    let help: String?
    /// "bool" | "int" | "string" | "enum"
    let type: String
    let value: String
    let minInt: Int?
    let maxInt: Int?
    let unit: String?
    let maxLength: Int?
    let options: [SettingOptionDTO]?

    var id: String { key }
}

struct SettingsSectionDTO: Codable, Equatable, Identifiable {
    let id: String
    let title: String
    let icon: String
    let fields: [SettingFieldDTO]
}

struct SettingsResponseDTO: Codable, Equatable {
    let serverType: String
    let serverName: String
    let serverRunning: Bool
    let editable: Bool
    let sections: [SettingsSectionDTO]
    let note: String?
}

struct SettingRejectionDTO: Codable, Equatable, Identifiable {
    let key: String
    let reason: String
    var id: String { key }
}

struct SettingsUpdateResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let restartRequired: Bool
    let appliedKeys: [String]
    let rejected: [SettingRejectionDTO]?
    let sections: [SettingsSectionDTO]?
}

// MARK: - Broadcast

struct BroadcastStatusDTO: Codable, Equatable {
    let xboxBroadcastRunning: Bool
    let bedrockBroadcastRunning: Bool
}

struct BroadcastAutoStartDTO: Codable, Equatable {
    let enabled: Bool
}

struct BroadcastAuthPromptDTO: Codable, Equatable, Identifiable {
    var id: String { code ?? "none" }
    let isPresent: Bool
    let code: String?
    let linkURL: String?
}

// MARK: - Session Log

struct SessionEventDTO: Codable, Identifiable, Equatable {
    let id: String
    let playerName: String
    let eventType: String   // "joined" or "left"
    let timestamp: String   // ISO8601
}

struct SessionLogResponseDTO: Codable, Equatable {
    let activeServerId: String?
    let events: [SessionEventDTO]
}

// MARK: - Player Profiles

struct ItemEnchantmentDTO: Codable, Equatable {
    let id: String
    let level: Int
    let displayName: String
}

struct InventoryItemDTO: Codable, Identifiable, Equatable {
    let slot: Int
    let itemID: String
    let iconName: String
    let count: Int
    let displayName: String
    let enchantments: [ItemEnchantmentDTO]
    let damage: Int

    var id: Int { slot }

    var itemTextureURL: URL? {
        URL(string: "https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.21.1/assets/minecraft/textures/item/\(iconName).png")
    }
    var blockTextureURL: URL? {
        URL(string: "https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.21.1/assets/minecraft/textures/block/\(iconName).png")
    }
}

struct PlayerStatsDTO: Codable, Equatable {
    let health: Float
    let maxHealth: Float
    let foodLevel: Int
    let xpLevel: Int
    let xpTotal: Int
    let gameMode: Int
    let gameModeDisplay: String
    let posX: Double
    let posY: Double
    let posZ: Double
    let dimensionDisplay: String
    let score: Int

    var healthFraction: Double {
        guard maxHealth > 0 else { return 0 }
        return Double(min(health, maxHealth)) / Double(maxHealth)
    }
    var foodFraction: Double { Double(min(max(foodLevel, 0), 20)) / 20.0 }
}

struct PlayerProfileDTO: Codable, Identifiable, Equatable {
    let id: String
    let username: String?
    let imageIdentifier: String
    let isOnline: Bool
    let isOp: Bool
    let lastSeen: String?
    let isBedrockPlayer: Bool
    let isHidden: Bool?
    let skinOverrideIdentifier: String?
    let hasSkinFileOverride: Bool?
    let stats: PlayerStatsDTO?
    let inventory: [InventoryItemDTO]

    var displayName: String { username ?? String(id.prefix(8)) + "…" }
    var isHiddenResolved: Bool { isHidden ?? false }
    var hasSkinOverride: Bool { skinOverrideIdentifier != nil || hasSkinFileOverride == true }

    var avatarURL: URL? {
        URL(string: "https://mc-heads.net/avatar/\(imageIdentifier)/64")
    }
}

struct PlayerProfilesResponseDTO: Codable, Equatable {
    let profiles: [PlayerProfileDTO]
    let isLoadingStats: Bool
}

struct PlayerSkinResponseDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let profileId: String?
    let imageBase64: String?
    let imageMimeType: String?
    let lookupIdentifier: String?
    let isOverride: Bool?
    let source: String?
}

struct PlayerSkinOverrideResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let profileId: String?
    let lookupIdentifier: String?
}

struct HiddenProfileMutationResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let profileId: String?
    let isHidden: Bool?
}

// MARK: - Server Files

struct ServerFileItemDTO: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let path: String
    let isDirectory: Bool
    let sizeBytes: Int64?
    let modifiedAt: String?
    let fileExtension: String?
    let isPreviewable: Bool?
}

struct ServerFilesResponseDTO: Codable, Equatable {
    let serverName: String?
    let path: String
    let parentPath: String?
    let items: [ServerFileItemDTO]
    let note: String?
}

struct ServerFileReadResponseDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let path: String?
    let name: String?
    let sizeBytes: Int64?
    let content: String?
    let encoding: String?
    let truncated: Bool?
}

// MARK: - Client Export

struct ClientExportItemDTO: Codable, Identifiable, Equatable {
    let id: String
    let fileName: String
    let displayName: String
    let iconURL: String?
    let projectURL: String?
    let clientStatus: String
    let statusSource: String
    let selectedByDefault: Bool
}

struct ClientExportResponseDTO: Codable, Equatable {
    let serverName: String?
    let serverType: String
    let exportKind: String
    let isPaperLike: Bool
    let items: [ClientExportItemDTO]
    let selectedCount: Int
    let shareText: String?
    let zipFileName: String?
    let zipBase64: String?
    let note: String?
}

// MARK: - World Slots

struct WorldSlotDTO: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let isActive: Bool
    let createdAt: String
    let zipSizeBytes: Int64?
    let worldSeed: String?
}

struct WorldSlotsResponseDTO: Codable, Equatable {
    let slots: [WorldSlotDTO]
    let activeSlotId: String?
    let serverRunning: Bool
    /// True while a Bedrock level.dat repair is running (POST /worlds/repair).
    /// Optional so older server builds (without the field) still decode.
    let isRepairing: Bool?
}

/// Result of a world-management mutation (create / rename / replace / repair).
/// `updated` echoes the fresh slot list so the client can refresh in one round-trip.
struct WorldMutationResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let updated: WorldSlotsResponseDTO?
}

// MARK: - Diagnostics: Health cards + startup problems (P10)

struct HealthCardDTO: Codable, Equatable, Identifiable {
    let id: String
    let title: String
    let shortLabel: String
    let severity: String          // "green" | "yellow" | "red" | "gray"
    let detail: String?
    let iconSystemName: String
    let actionLabel: String?
    let actionCode: String?
}

struct HealthResponseDTO: Codable, Equatable {
    let serverType: String
    let serverName: String
    let serverRunning: Bool
    let overallSeverity: String
    let cards: [HealthCardDTO]
    let note: String?
}

struct StartupProblemDTO: Codable, Equatable, Identifiable {
    let id: String
    let kind: String
    let kindTitle: String
    let iconSystemName: String
    let offenderName: String
    let requirement: String?
    let installedFile: String?
    let installedJarStem: String?
    let missingDependency: String?
    let rawExcerpt: String
    let isRepairing: Bool
    let availableActions: [String]
    let modrinthURL: String?
}

struct HealthProblemsResponseDTO: Codable, Equatable {
    let serverType: String
    let serverRunning: Bool
    let isSoftFail: Bool
    let problems: [StartupProblemDTO]
    let note: String?
}

struct HealthRepairResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let updated: HealthProblemsResponseDTO?
}

// MARK: - Connectivity (P11)

struct ConnectivityPlayitDTO: Codable, Equatable {
    let enabled: Bool
    let running: Bool
    let address: String?
}

struct ConnectivityBroadcastDTO: Codable, Equatable {
    let xboxRunning: Bool
    let bedrockRunning: Bool
}

struct ConnectivityResponseDTO: Codable, Equatable {
    let serverType: String
    let serverName: String
    let serverRunning: Bool
    let status: String        // reachable | unreachable | offline | starting | unknown
    let severity: String      // green | yellow | red | gray
    let headline: String
    let detail: String?
    let joinAddress: String?
    let method: String        // playit | duckdns | public-ip | none
    let localListening: Bool?
    let externallyReachable: Bool?
    let playersOnline: Int?
    let playersMax: Int?
    let motd: String?
    let playit: ConnectivityPlayitDTO?
    let broadcast: ConnectivityBroadcastDTO?
    let note: String?
}

// MARK: - Backups

struct BackupItemDTO: Codable, Identifiable, Equatable {
    let id: String
    let displayName: String
    let fileSize: Int64?
    let modificationDate: String?
    let isAutomatic: Bool
    let slotId: String?
    let slotName: String?
    let triggerReason: String
}

struct BackupsResponseDTO: Codable, Equatable {
    let backups: [BackupItemDTO]
}

struct BackupConfigResponseDTO: Codable, Equatable {
    let serverName: String
    let autoBackupEnabled: Bool
    let autoBackupIntervalMinutes: Int
    let autoBackupMaxCount: Int
    let intervalOptions: [Int]
    let note: String?
}

struct BackupConfigUpdateResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let config: BackupConfigResponseDTO?
}

// MARK: - Resource Packs

struct ResourcePackItemDTO: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let fileName: String
    let fileSizeDisplay: String
    let packKind: String   // "java" | "bedrock" | "geyser"
    let isActive: Bool
    let typeLabel: String
}

struct ResourcePacksResponseDTO: Codable, Equatable {
    let serverType: String
    let isJava: Bool
    let packs: [ResourcePackItemDTO]
    let geyserPacks: [ResourcePackItemDTO]
    let isGeyserAvailable: Bool
    let activePackUrl: String?
    let requirePack: Bool
    let note: String?
}

struct ResourcePackMutationResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let updated: ResourcePacksResponseDTO?
}

// MARK: - Versions (server JAR / version picker)

struct VersionEntryDTO: Codable, Identifiable, Equatable {
    let id: String
    let displayLabel: String
    let mcVersion: String
    let loaderVersion: String?
    let buildLabel: String?
    let isStable: Bool
    let isLatest: Bool
}

struct VersionsResponseDTO: Codable, Equatable {
    let supportsVersions: Bool
    let flavorName: String
    let currentVersion: String?
    let isBedrock: Bool
    let versions: [VersionEntryDTO]
    let note: String?
}

struct VersionChangeResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let requiresRestart: Bool
}

// MARK: - Playit tunnel (P12)

struct PlayitStatusResponseDTO: Codable, Equatable {
    let serverName: String
    let serverType: String
    let playitEnabled: Bool
    let isRunning: Bool
    let hasSecretKey: Bool
    let javaAddress: String?
    let bedrockAddress: String?
    let voiceAddress: String?
    let voiceChatEnabled: Bool
    let note: String?
}

struct PlayitActionResultDTO: Codable {
    let result: String
    let message: String?
}

// MARK: - DuckDNS (P13)

struct DuckDNSStatusResponseDTO: Codable, Equatable {
    let hostname: String?
    let isConfigured: Bool
}

struct DuckDNSUpdateResultDTO: Codable {
    let success: Bool
    let hostname: String?
    let message: String?
}

// MARK: - Geyser config (P13)

struct GeyserConfigResponseDTO: Codable, Equatable {
    let serverName: String
    let serverType: String
    let isGeyserInstalled: Bool
    let address: String?
    let port: Int?
    let configFileExists: Bool
    let note: String?
}

struct GeyserConfigUpdateResultDTO: Codable {
    let success: Bool
    let message: String
    let address: String?
    let port: Int?
}

// MARK: - RAM allocation (/config/ram)

struct RAMConfigResponseDTO: Codable, Equatable {
    let serverName: String
    let serverType: String
    let minRamGB: Double
    let maxRamGB: Double
    let physicalRAMGB: Int
    let recommendedMaxGB: Int
    let serverRunning: Bool
    let hasActiveServer: Bool

    var isBedrock: Bool { serverType == "bedrock" }
}

struct RAMConfigUpdateResultDTO: Codable {
    let success: Bool
    let minRamGB: Double?
    let maxRamGB: Double?
    let restartRequired: Bool
    let message: String?
}

// MARK: - User management (P17)

struct MeResponseDTO: Codable {
    let role: String
    let name: String?
    let permissions: [String]?
    let isNamedToken: Bool?
}

struct UserSummaryDTO: Codable, Identifiable {
    let id: String
    let label: String
    let role: String
    let permissions: [String]?
    let createdAtISO8601: String?
    let expiresAtISO8601: String?
    let isExpired: Bool
}

struct UserListResponseDTO: Codable {
    let users: [UserSummaryDTO]
}

struct UserCreateResultDTO: Codable, Identifiable {
    let success: Bool
    let message: String
    let user: UserSummaryDTO?
    let token: String?
    var id: String { user?.id ?? message }
}

struct UserRevokeResultDTO: Codable {
    let success: Bool
    let message: String
}

struct UserUpdateResultDTO: Codable {
    let success: Bool
    let message: String
    let user: UserSummaryDTO?
}

// MARK: - Java Runtimes

struct JavaRuntimeDTO: Codable, Identifiable, Equatable {
    let name: String
    let executablePath: String
    let majorVersion: Int?

    var id: String { executablePath }
    var versionLabel: String { majorVersion.map { "Java \($0)" } ?? "Java" }
}

struct JavaRuntimesResponseDTO: Codable, Equatable {
    let runtimes: [JavaRuntimeDTO]
}

struct BroadcastJarStatusDTO: Codable, Equatable {
    let installed: Bool
    let downloading: Bool
    let filename: String?
}

struct BroadcastJarDownloadResultDTO: Codable, Equatable {
    let success: Bool
    let message: String
    let filename: String?
}

struct JavaConfigResponseDTO: Codable, Equatable {
    let executablePath: String?
}

struct JavaConfigSetRequest: Codable {
    let executablePath: String?
}
