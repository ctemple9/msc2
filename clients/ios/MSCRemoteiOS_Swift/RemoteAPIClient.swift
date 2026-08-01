import Foundation

enum RemoteAPIError: LocalizedError {
    case invalidBaseURL
    case insecureHTTPNotAllowed
    case insecureWebSocketNotAllowed
    case missingToken
    case httpStatus(Int, String?)
    case decodingFailed
    case network(String)

    var errorDescription: String? {
        switch self {
        case .invalidBaseURL:
            return "Base URL is invalid."
        case .insecureHTTPNotAllowed:
            return "Blocked: HTTP is only allowed for local/private addresses. Use LAN/VPN or HTTPS."
        case .insecureWebSocketNotAllowed:
            return "Blocked: WS is only allowed for local/private addresses. Use LAN/VPN or WSS."
        case .missingToken:
            return "Token is missing."
        case .httpStatus(let code, let body):
            if let body, !body.isEmpty {
                return "HTTP \(code): \(body)"
            }
            return "HTTP \(code)."
        case .decodingFailed:
            return "Failed to decode server response."
        case .network(let msg):
            return "Network error: \(msg)"
        }
    }
}

final class RemoteAPIClient {
    private let baseURL: URL
    private let token: String

    /// Used for all standard HTTP requests. Short timeouts are appropriate
    /// here — if the server doesn't respond to a status or command request
    /// in 10 seconds, something is genuinely wrong.
    private let session: URLSession

    /// Used exclusively for WebSocket connections. WebSocket connections are
    /// long-lived and may go silent for extended periods between console
    /// messages. Applying timeoutIntervalForRequest (the between-packet
    /// idle timeout) to a WebSocket is incorrect — it will kill a healthy
    /// idle socket after 3 seconds of server silence, which is exactly the
    /// "stream dies immediately" bug. This session has no timeouts so
    /// URLSession never tears it down due to inactivity.
    private let wsSession: URLSession

    /// Used only for add-on installs. Installs download a jar (plus any required
    /// dependencies) on the Mac and can legitimately take much longer than a status
    /// request, so the standard 15s cap would spuriously "fail" a running install.
    /// A generous timeout keeps the connection open long enough to receive the
    /// authoritative success/failure result for essentially all single-addon installs.
    private let installSession: URLSession

    /// GET /health runs live diagnostic checks on the Mac (java -version subprocess, a
    /// local port probe, and an external reachability ping with its own ~10s timeout), so
    /// the standard 15s cap is too tight. This session gives it comfortable headroom.
    private let diagnosticsSession: URLSession

    init(baseURL: URL, token: String) throws {
        guard baseURL.scheme != nil, baseURL.host != nil else { throw RemoteAPIError.invalidBaseURL }
        guard NetworkSafety.httpIsAllowed(for: baseURL) else { throw RemoteAPIError.insecureHTTPNotAllowed }
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.missingToken }

        self.baseURL = baseURL
        self.token = trimmed

        let httpConfig = URLSessionConfiguration.ephemeral
        httpConfig.timeoutIntervalForRequest = 10
        httpConfig.timeoutIntervalForResource = 15
        self.session = URLSession(configuration: httpConfig)

        // No timeouts — the WebSocket stays open until explicitly cancelled.
        let wsConfig = URLSessionConfiguration.ephemeral
        wsConfig.timeoutIntervalForRequest = .infinity
        wsConfig.timeoutIntervalForResource = .infinity
        self.wsSession = URLSession(configuration: wsConfig)

        // Install downloads can take a while; give them a generous window.
        let installConfig = URLSessionConfiguration.ephemeral
        installConfig.timeoutIntervalForRequest = 120
        installConfig.timeoutIntervalForResource = 180
        self.installSession = URLSession(configuration: installConfig)

        let diagConfig = URLSessionConfiguration.ephemeral
        diagConfig.timeoutIntervalForRequest = 30
        diagConfig.timeoutIntervalForResource = 40
        self.diagnosticsSession = URLSession(configuration: diagConfig)
    }

    // MARK: - Role / identity

    struct MeResponse: Decodable {
        let role: String
        let name: String?
        let permissions: [String]?
        let isNamedToken: Bool?
    }

    func getMe() async throws -> MeResponse {
        try await get(path: "/me", query: [:], as: MeResponse.self)
    }

    // MARK: - User management (P17)

    func getUsers() async throws -> UserListResponseDTO {
        try await get(path: "/users", query: [:], as: UserListResponseDTO.self)
    }

    private struct CreateUserBody: Encodable {
        let label: String
        let role: String
        let permissions: [String]?
        let expiresInDays: Int?
    }

    func createUser(label: String, role: String = "admin", permissions: [String]? = nil, expiresInDays: Int? = nil) async throws -> UserCreateResultDTO {
        let body = CreateUserBody(label: label, role: role, permissions: permissions, expiresInDays: expiresInDays)
        return try await post(path: "/users", body: body, as: UserCreateResultDTO.self)
    }

    private struct UserIdBody: Encodable { let userId: String }

    func revokeUser(userId: String) async throws -> UserRevokeResultDTO {
        try await post(path: "/users/revoke", body: UserIdBody(userId: userId), as: UserRevokeResultDTO.self)
    }

    private struct UpdateUserBody: Encodable {
        let userId: String
        let label: String?
        let role: String?
        let permissions: [String]?
        let expiresInDays: Int?
    }

    func updateUser(userId: String, label: String? = nil, role: String? = nil, permissions: [String]? = nil, expiresInDays: Int? = nil) async throws -> UserUpdateResultDTO {
        let body = UpdateUserBody(userId: userId, label: label, role: role, permissions: permissions, expiresInDays: expiresInDays)
        return try await post(path: "/users/update", body: body, as: UserUpdateResultDTO.self)
    }

    // MARK: - Read-only endpoints

    func getStatus() async throws -> RemoteAPIStatus {
        try await get(path: "/status", query: [:], as: RemoteAPIStatus.self)
    }

    func getServers() async throws -> [ServerDTO] {
        try await get(path: "/servers", query: [:], as: [ServerDTO].self)
    }

    func getConsoleTail(n: Int) async throws -> [ConsoleLineDTO] {
        let clamped = max(1, min(2000, n))
        return try await get(path: "/console/tail", query: ["n": "\(clamped)"], as: [ConsoleLineDTO].self)
    }

    // MARK: - Performance snapshot

    /// Recommended server endpoint:
    /// GET /performance  -> PerformanceSnapshotDTO
    func getPerformanceSnapshot() async throws -> PerformanceSnapshotDTO {
        try await get(path: "/performance", query: [:], as: PerformanceSnapshotDTO.self)
    }

    // MARK: - Players snapshot

    /// GET /players -> PlayersResponse
    /// Returns the online player list from the macOS app.
    /// Returns { players: [], count: 0 } if the server is not running.
    func getPlayers() async throws -> PlayersResponse {
        try await get(path: "/players", query: [:], as: PlayersResponse.self)
    }
    // MARK: - Control endpoints

    func setActiveServer(serverId: String) async throws -> SimpleResult {
        let trimmed = serverId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing server id.") }
        return try await post(path: "/active-server", body: ActiveServerRequest(serverId: trimmed), as: SimpleResult.self)
    }

    func renameServer(serverId: String, name: String) async throws -> ServerRenameResultDTO {
        let trimmedId = serverId.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else { throw RemoteAPIError.network("Missing server id.") }
        guard !trimmedName.isEmpty else { throw RemoteAPIError.network("Server name is empty.") }
        return try await post(path: "/servers/rename",
                              body: ServerRenameRequest(serverId: trimmedId, name: trimmedName),
                              as: ServerRenameResultDTO.self)
    }

    func deleteServer(serverId: String) async throws -> ServerDeleteResultDTO {
        let trimmed = serverId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing server id.") }
        return try await post(path: "/servers/delete",
                              body: ServerDeleteRequest(serverId: trimmed),
                              as: ServerDeleteResultDTO.self)
    }

    func createFreshServer(name: String, serverType: ServerType, javaFlavor: RemoteJavaServerFlavor?,
                           selectedVersion: VersionEntryDTO?, port: Int, maxPlayers: Int?,
                           enableCrossPlay: Bool, crossPlayBedrockPort: Int, enablePlayit: Bool,
                           enableXboxBroadcast: Bool, difficulty: String, gamemode: String,
                           worldName: String?, worldSeed: String?, acceptEula: Bool,
                           bedrockVersion: String?, dockerImage: String?,
                           javaPath: String? = nil) async throws -> ServerCreateResultDTO {
        try await post(path: "/servers/create",
                       body: ServerCreateRequest(name: name, serverType: serverType.rawValue,
                                                 javaFlavor: javaFlavor?.rawValue,
                                                 port: port, maxPlayers: maxPlayers,
                                                 enableCrossPlay: enableCrossPlay,
                                                 crossPlayBedrockPort: crossPlayBedrockPort,
                                                 enablePlayit: enablePlayit,
                                                 enableXboxBroadcast: enableXboxBroadcast,
                                                 difficulty: difficulty, gamemode: gamemode,
                                                 worldName: worldName, worldSeed: worldSeed,
                                                 versionId: selectedVersion?.id,
                                                 minecraftVersion: selectedVersion?.mcVersion,
                                                 loaderVersion: selectedVersion?.loaderVersion,
                                                 acceptEula: acceptEula,
                                                 bedrockVersion: bedrockVersion ?? (serverType == .bedrock ? selectedVersion?.id : nil),
                                                 dockerImage: dockerImage,
                                                 javaPath: javaPath),
                       as: ServerCreateResultDTO.self,
                       urlSession: installSession)
    }

    func getBroadcastJarStatus() async throws -> BroadcastJarStatusDTO {
        try await get(path: "/broadcast/jar-status", query: [:], as: BroadcastJarStatusDTO.self)
    }

    func downloadBroadcastJar() async throws -> BroadcastJarDownloadResultDTO {
        try await post(path: "/broadcast/download-jar", body: EmptyBody(), as: BroadcastJarDownloadResultDTO.self,
                       urlSession: installSession)
    }

    func getJavaRuntimes() async throws -> JavaRuntimesResponseDTO {
        try await get(path: "/java-runtimes", query: [:], as: JavaRuntimesResponseDTO.self)
    }

    func getJavaConfig() async throws -> JavaConfigResponseDTO {
        try await get(path: "/config/java-runtime", query: [:], as: JavaConfigResponseDTO.self)
    }

    func setJavaConfig(executablePath: String?) async throws {
        let body = JavaConfigSetRequest(executablePath: executablePath)
        _ = try await post(path: "/config/java-runtime", body: body, as: JavaConfigResponseDTO.self)
    }

    func acceptServerEULA(serverId: String) async throws -> ServerEULAResultDTO {
        let trimmed = serverId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing server id.") }
        return try await post(path: "/servers/eula",
                              body: ServerEULARequest(serverId: trimmed),
                              as: ServerEULAResultDTO.self)
    }

    func scanServerImport(sourcePath: String, importKind: String) async throws -> ServerImportScanResponseDTO {
        try await post(path: "/servers/import",
                       body: ServerImportRequest(action: "scan", sourcePath: sourcePath, importKind: importKind,
                                                 displayName: nil, serverType: nil, activeWorldName: nil,
                                                 port: nil, maxPlayers: nil, acceptEula: nil,
                                                 enablePlayit: nil, transferMode: nil, backupPath: nil,
                                                 javaPortOverrides: nil, bedrockPortOverrides: nil),
                       as: ServerImportScanResponseDTO.self,
                       urlSession: installSession)
    }

    func importExistingServer(sourcePath: String, importKind: String, displayName: String, serverType: ServerType,
                              activeWorldName: String?, port: Int?, maxPlayers: Int?, acceptEula: Bool,
                              enablePlayit: Bool) async throws -> ServerImportResultDTO {
        try await post(path: "/servers/import",
                       body: ServerImportRequest(action: "importExisting", sourcePath: sourcePath,
                                                 importKind: importKind, displayName: displayName,
                                                 serverType: serverType.rawValue,
                                                 activeWorldName: activeWorldName, port: port,
                                                 maxPlayers: maxPlayers, acceptEula: acceptEula,
                                                 enablePlayit: enablePlayit, transferMode: nil,
                                                 backupPath: nil, javaPortOverrides: nil,
                                                 bedrockPortOverrides: nil),
                       as: ServerImportResultDTO.self,
                       urlSession: installSession)
    }

    func importTransferPackage(sourcePath: String, replaceAll: Bool, backupPath: String?) async throws -> ServerImportResultDTO {
        try await post(path: "/servers/import",
                       body: ServerImportRequest(action: "importTransfer", sourcePath: sourcePath,
                                                 importKind: "transfer", displayName: nil, serverType: nil,
                                                 activeWorldName: nil, port: nil, maxPlayers: nil,
                                                 acceptEula: nil, enablePlayit: nil,
                                                 transferMode: replaceAll ? "replaceAll" : "merge",
                                                 backupPath: backupPath, javaPortOverrides: nil,
                                                 bedrockPortOverrides: nil),
                       as: ServerImportResultDTO.self,
                       urlSession: installSession)
    }

    func start() async throws -> SimpleResult {
        try await post(path: "/start", body: EmptyBody(), as: SimpleResult.self)
    }

    func stop() async throws -> SimpleResult {
        try await post(path: "/stop", body: EmptyBody(), as: SimpleResult.self)
    }

    func sendCommand(_ command: String) async throws -> CommandResult {
        let trimmed = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Command is empty.") }
        return try await post(path: "/command", body: CommandRequest(command: trimmed), as: CommandResult.self)
    }

    // MARK: - Components

    func getComponents() async throws -> ComponentsStatusDTO {
        try await get(path: "/components", query: [:], as: ComponentsStatusDTO.self)
    }

    func updateComponent(_ component: String) async throws -> ComponentUpdateResultDTO {
        try await post(path: "/components/update",
                       body: ComponentUpdateRequest(component: component),
                       as: ComponentUpdateResultDTO.self)
    }

    // MARK: - Add-ons

    func getAddons() async throws -> AddonsResponseDTO {
        try await get(path: "/addons", query: [:], as: AddonsResponseDTO.self)
    }

    func updateAddon(jarStem: String) async throws -> AddonUpdateResultDTO {
        let trimmed = jarStem.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing jarStem.") }
        return try await post(path: "/components/update",
                              body: AddonUpdateRequest(jarStem: trimmed, updateAll: false),
                              as: AddonUpdateResultDTO.self)
    }

    func updateAllAddons() async throws -> AddonUpdateResultDTO {
        try await post(path: "/components/update",
                       body: AddonUpdateRequest(jarStem: nil, updateAll: true),
                       as: AddonUpdateResultDTO.self)
    }

    func removeAddon(jarStem: String) async throws -> AddonRemoveResultDTO {
        let trimmed = jarStem.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing jarStem.") }
        return try await post(path: "/components/remove",
                              body: AddonRemoveRequest(jarStem: trimmed),
                              as: AddonRemoveResultDTO.self)
    }

    // MARK: - Catalog (search + install)

    func searchCatalog(query: String, offset: Int = 0) async throws -> CatalogSearchResponseDTO {
        var q: [String: String] = ["offset": String(offset)]
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty { q["q"] = trimmed }
        return try await get(path: "/catalog/search", query: q, as: CatalogSearchResponseDTO.self)
    }

    /// Installs a catalog add-on. Uses the long-timeout install session because the
    /// Mac downloads the jar (and dependencies) before responding with the result.
    func installAddon(projectId: String, slug: String, title: String) async throws -> CatalogInstallResultDTO {
        try await post(path: "/components/install",
                       body: CatalogInstallRequest(projectId: projectId, slug: slug, title: title),
                       as: CatalogInstallResultDTO.self,
                       urlSession: installSession)
    }

    // MARK: - Versions (server JAR picker)

    func getVersions() async throws -> VersionsResponseDTO {
        try await get(path: "/versions", query: [:], as: VersionsResponseDTO.self)
    }

    func getCreateVersions(serverType: ServerType, javaFlavor: RemoteJavaServerFlavor?) async throws -> VersionsResponseDTO {
        var query = ["serverType": serverType.rawValue]
        if serverType == .java, let javaFlavor {
            query["javaFlavor"] = javaFlavor.rawValue
        }
        return try await get(path: "/versions/create", query: query, as: VersionsResponseDTO.self)
    }

    /// Changes the server JAR / version. Uses the install session because the Mac downloads
    /// the JAR (or runs the NeoForge/Forge installer) before responding.
    func changeVersion(versionId: String, loaderVersion: String? = nil) async throws -> VersionChangeResultDTO {
        struct ChangeVersionRequest: Encodable {
            let versionId: String
            let loaderVersion: String?
        }
        return try await post(path: "/components/version",
                              body: ChangeVersionRequest(versionId: versionId, loaderVersion: loaderVersion),
                              as: VersionChangeResultDTO.self,
                              urlSession: installSession)
    }

    // MARK: - Resource Packs

    func getResourcePacks() async throws -> ResourcePacksResponseDTO {
        try await get(path: "/resourcepacks", query: [:], as: ResourcePacksResponseDTO.self)
    }

    /// Activate a local pack by ID, or pass nil to clear the active pack.
    func activateResourcePack(packId: String?, require: Bool = false) async throws -> ResourcePackMutationResultDTO {
        struct Body: Encodable { let packId: String?; let require: Bool }
        return try await post(path: "/resourcepacks/activate",
                              body: Body(packId: packId, require: require),
                              as: ResourcePackMutationResultDTO.self)
    }

    /// Write a custom URL directly to server.properties (Java "add by URL").
    func setResourcePackURL(url: String, sha1: String? = nil, require: Bool = false) async throws -> ResourcePackMutationResultDTO {
        struct Body: Encodable { let url: String; let sha1: String?; let require: Bool }
        return try await post(path: "/resourcepacks/seturl",
                              body: Body(url: url, sha1: sha1, require: require),
                              as: ResourcePackMutationResultDTO.self)
    }

    /// Enable or disable a Geyser pack.
    func toggleGeyserPack(packId: String, enabled: Bool) async throws -> ResourcePackMutationResultDTO {
        struct Body: Encodable { let packId: String; let enabled: Bool }
        return try await post(path: "/resourcepacks/toggle",
                              body: Body(packId: packId, enabled: enabled),
                              as: ResourcePackMutationResultDTO.self)
    }

    /// Remove a pack from disk.
    func removeResourcePack(packId: String, packKind: String) async throws -> ResourcePackMutationResultDTO {
        struct Body: Encodable { let packId: String; let packKind: String }
        return try await post(path: "/resourcepacks/remove",
                              body: Body(packId: packId, packKind: packKind),
                              as: ResourcePackMutationResultDTO.self)
    }

    // MARK: - Settings (typed server.properties)

    func getSettings() async throws -> SettingsResponseDTO {
        try await get(path: "/settings", query: [:], as: SettingsResponseDTO.self)
    }

    /// Applies a sparse map of changed keys. Returns the result (with echoed fresh schema).
    func updateSettings(changes: [String: String]) async throws -> SettingsUpdateResultDTO {
        try await post(path: "/settings",
                       body: SettingsUpdateRequest(changes: changes),
                       as: SettingsUpdateResultDTO.self)
    }

    // MARK: - Allowlist (Bedrock)

    /// GET /allowlist -> AllowlistResponseDTO.
    /// Returns serverType so the caller can surface the UI for Bedrock only.
    func getAllowlist() async throws -> AllowlistResponseDTO {
        try await get(path: "/allowlist", query: [:], as: AllowlistResponseDTO.self)
    }

    /// POST /allowlist -> AllowlistMutationResultDTO. `action` is "add" or "remove".
    func mutateAllowlist(action: String, name: String) async throws -> AllowlistMutationResultDTO {
        let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedName.isEmpty else { throw RemoteAPIError.network("Gamertag is empty.") }
        return try await post(path: "/allowlist",
                              body: AllowlistMutationRequest(action: action, name: trimmedName),
                              as: AllowlistMutationResultDTO.self)
    }

    // MARK: - Broadcast

    func getBroadcastStatus() async throws -> BroadcastStatusDTO {
        try await get(path: "/broadcast/status", query: [:], as: BroadcastStatusDTO.self)
    }

    func restartBroadcast() async throws -> SimpleResult {
        try await post(path: "/broadcast/restart", body: EmptyBody(), as: SimpleResult.self)
    }

    func updateBroadcastCredentials(email: String, password: String, gamertag: String) async throws -> SimpleResult {
        try await post(path: "/broadcast/credentials",
                       body: BroadcastCredentialsRequest(email: email, password: password, gamertag: gamertag),
                       as: SimpleResult.self)
    }

    func getBroadcastAutoStart() async throws -> BroadcastAutoStartDTO {
        try await get(path: "/broadcast/autostart", query: [:], as: BroadcastAutoStartDTO.self)
    }

    func setBroadcastAutoStart(enabled: Bool) async throws -> SimpleResult {
        struct Body: Encodable { let enabled: Bool }
        return try await post(path: "/broadcast/autostart", body: Body(enabled: enabled), as: SimpleResult.self)
    }

    func startBroadcast() async throws -> SimpleResult {
        try await post(path: "/broadcast/start", body: EmptyBody(), as: SimpleResult.self)
    }

    func stopBroadcast() async throws -> SimpleResult {
        try await post(path: "/broadcast/stop", body: EmptyBody(), as: SimpleResult.self)
    }

    func getAuthPrompt() async throws -> BroadcastAuthPromptDTO {
        try await get(path: "/broadcast/auth-prompt", query: [:], as: BroadcastAuthPromptDTO.self)
    }

    func dismissAuthPrompt() async throws {
        _ = try await post(path: "/broadcast/auth-prompt/dismiss", body: EmptyBody(), as: SimpleResult.self)
    }

    // MARK: - Session Log

    func getSessionLog() async throws -> SessionLogResponseDTO {
        try await get(path: "/session-log", query: [:], as: SessionLogResponseDTO.self)
    }

    // MARK: - Player Profiles

    func getPlayerProfiles() async throws -> PlayerProfilesResponseDTO {
        try await get(path: "/players/profiles", query: [:], as: PlayerProfilesResponseDTO.self)
    }

    func getPlayerSkin(profileId: String) async throws -> PlayerSkinResponseDTO {
        let trimmed = profileId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing profile id.") }
        let encoded = trimmed.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? trimmed
        return try await get(path: "/players/\(encoded)/skin", query: [:], as: PlayerSkinResponseDTO.self)
    }

    func setPlayerSkinOverride(profileId: String, lookupIdentifier: String?) async throws -> PlayerSkinOverrideResultDTO {
        let trimmed = profileId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing profile id.") }
        let lookup = lookupIdentifier?.trimmingCharacters(in: .whitespacesAndNewlines)
        return try await post(path: "/players/skin-override",
                              body: PlayerSkinOverrideRequest(profileId: trimmed,
                                                              lookupIdentifier: lookup?.isEmpty == false ? lookup : nil),
                              as: PlayerSkinOverrideResultDTO.self)
    }

    func setHiddenProfile(profileId: String, hidden: Bool) async throws -> HiddenProfileMutationResultDTO {
        let trimmed = profileId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing profile id.") }
        return try await post(path: "/players/hidden",
                              body: HiddenProfileMutationRequest(profileId: trimmed, hidden: hidden),
                              as: HiddenProfileMutationResultDTO.self)
    }

    // MARK: - Server files

    func getServerFiles(path: String?) async throws -> ServerFilesResponseDTO {
        let trimmed = path?.trimmingCharacters(in: .whitespacesAndNewlines)
        let query = (trimmed?.isEmpty == false) ? ["path": trimmed!] : [:]
        return try await get(path: "/files", query: query, as: ServerFilesResponseDTO.self)
    }

    func readServerFile(path: String) async throws -> ServerFileReadResponseDTO {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing file path.") }
        return try await get(path: "/files/read", query: ["path": trimmed], as: ServerFileReadResponseDTO.self)
    }

    // MARK: - Client export

    func getClientExport(selectedIds: [String]? = nil) async throws -> ClientExportResponseDTO {
        let selected = selectedIds?.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty } ?? []
        let query = selected.isEmpty ? [:] : ["selected": selected.joined(separator: ",")]
        return try await get(path: "/components/client-export", query: query, as: ClientExportResponseDTO.self, urlSession: installSession)
    }

    // MARK: - Worlds

    func getWorlds() async throws -> WorldSlotsResponseDTO {
        try await get(path: "/worlds", query: [:], as: WorldSlotsResponseDTO.self)
    }

    func activateWorldSlot(slotId: String) async throws -> SimpleResult {
        let trimmed = slotId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing slot id.") }
        return try await post(path: "/worlds/activate", body: ActivateSlotRequest(slotId: trimmed), as: SimpleResult.self)
    }

    /// Create a fresh (empty) named world slot that generates on first activation.
    func createWorld(name: String, seed: String?) async throws -> WorldMutationResultDTO {
        struct Body: Encodable { let name: String; let seed: String? }
        let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedName.isEmpty else { throw RemoteAPIError.network("Missing world name.") }
        let trimmedSeed = seed?.trimmingCharacters(in: .whitespacesAndNewlines)
        return try await post(path: "/worlds/create",
                              body: Body(name: trimmedName, seed: (trimmedSeed?.isEmpty == false) ? trimmedSeed : nil),
                              as: WorldMutationResultDTO.self)
    }

    /// Rename a saved world slot (metadata only).
    func renameWorld(slotId: String, name: String) async throws -> WorldMutationResultDTO {
        struct Body: Encodable { let slotId: String; let name: String }
        let trimmedId = slotId.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else { throw RemoteAPIError.network("Missing slot id.") }
        guard !trimmedName.isEmpty else { throw RemoteAPIError.network("Missing world name.") }
        return try await post(path: "/worlds/rename",
                              body: Body(slotId: trimmedId, name: trimmedName),
                              as: WorldMutationResultDTO.self)
    }

    /// Overwrite a slot's saved world with another saved slot's world (destructive to the target).
    func replaceWorld(slotId: String, sourceSlotId: String) async throws -> WorldMutationResultDTO {
        struct Body: Encodable { let slotId: String; let sourceSlotId: String }
        let trimmedId = slotId.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedSource = sourceSlotId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty, !trimmedSource.isEmpty else { throw RemoteAPIError.network("Missing slot id.") }
        return try await post(path: "/worlds/replace",
                              body: Body(slotId: trimmedId, sourceSlotId: trimmedSource),
                              as: WorldMutationResultDTO.self)
    }

    /// Repair the active Bedrock world's level.dat (long-running; poll GET /worlds for isRepairing).
    func repairWorld(slotId: String) async throws -> WorldMutationResultDTO {
        struct Body: Encodable { let slotId: String }
        let trimmedId = slotId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else { throw RemoteAPIError.network("Missing slot id.") }
        return try await post(path: "/worlds/repair",
                              body: Body(slotId: trimmedId),
                              as: WorldMutationResultDTO.self)
    }

    // MARK: - Diagnostics (health cards + startup problems)

    /// Runs the Mac's diagnostic health checks and returns the cards. Uses the longer
    /// diagnostics session because the Mac performs live subprocess + network probes.
    func getHealth() async throws -> HealthResponseDTO {
        try await get(path: "/health", query: [:], as: HealthResponseDTO.self, urlSession: diagnosticsSession)
    }

    func getHealthProblems() async throws -> HealthProblemsResponseDTO {
        try await get(path: "/health/problems", query: [:], as: HealthProblemsResponseDTO.self)
    }

    /// Reports whether the active server is joinable right now. Uses the longer diagnostics
    /// session because the Mac runs a live external reachability probe (~10s).
    func getConnectivity() async throws -> ConnectivityResponseDTO {
        try await get(path: "/connectivity", query: [:], as: ConnectivityResponseDTO.self, urlSession: diagnosticsSession)
    }

    // MARK: - Playit tunnel (P12)

    func getPlayitStatus() async throws -> PlayitStatusResponseDTO {
        try await get(path: "/playit", query: [:], as: PlayitStatusResponseDTO.self)
    }

    func startPlayit() async throws -> PlayitActionResultDTO {
        try await post(path: "/playit/start", body: EmptyBody(), as: PlayitActionResultDTO.self)
    }

    func stopPlayit() async throws -> PlayitActionResultDTO {
        try await post(path: "/playit/stop", body: EmptyBody(), as: PlayitActionResultDTO.self)
    }

    // MARK: - DuckDNS (P13)

    func getDuckDNSStatus() async throws -> DuckDNSStatusResponseDTO {
        try await get(path: "/duckdns", query: [:], as: DuckDNSStatusResponseDTO.self)
    }

    func updateDuckDNS(hostname: String?) async throws -> DuckDNSUpdateResultDTO {
        struct Body: Encodable { let hostname: String? }
        return try await post(path: "/duckdns", body: Body(hostname: hostname), as: DuckDNSUpdateResultDTO.self)
    }

    // MARK: - Geyser config (P13)

    func getGeyserConfig() async throws -> GeyserConfigResponseDTO {
        try await get(path: "/config/geyser", query: [:], as: GeyserConfigResponseDTO.self)
    }

    func updateGeyserConfig(address: String?, port: Int?) async throws -> GeyserConfigUpdateResultDTO {
        struct Body: Encodable { let address: String?; let port: Int? }
        return try await post(path: "/config/geyser", body: Body(address: address, port: port), as: GeyserConfigUpdateResultDTO.self)
    }

    // MARK: - RAM allocation (/config/ram)

    func getRAMConfig() async throws -> RAMConfigResponseDTO {
        try await get(path: "/config/ram", query: [:], as: RAMConfigResponseDTO.self)
    }

    func updateRAMConfig(minRamGB: Double?, maxRamGB: Double?) async throws -> RAMConfigUpdateResultDTO {
        struct Body: Encodable { let minRamGB: Double?; let maxRamGB: Double? }
        return try await post(path: "/config/ram",
                              body: Body(minRamGB: minRamGB, maxRamGB: maxRamGB),
                              as: RAMConfigUpdateResultDTO.self)
    }

    /// Triggers a repair action ("update" | "install" | "disable" | "delete") for a problem.
    func repairHealthProblem(problemId: String, action: String) async throws -> HealthRepairResultDTO {
        struct Body: Encodable { let problemId: String; let action: String }
        let trimmed = problemId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing problem id.") }
        return try await post(path: "/health/repair",
                              body: Body(problemId: trimmed, action: action),
                              as: HealthRepairResultDTO.self)
    }

    // MARK: - Backups

    func getBackups() async throws -> BackupsResponseDTO {
        try await get(path: "/backups", query: [:], as: BackupsResponseDTO.self)
    }

    func createBackupNow() async throws -> SimpleResult {
        try await post(path: "/backups/now", body: EmptyBody(), as: SimpleResult.self)
    }

    func restoreBackup(backupId: String) async throws -> SimpleResult {
        let trimmed = backupId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw RemoteAPIError.network("Missing backup id.") }
        return try await post(path: "/backups/restore", body: RestoreBackupRequest(backupId: trimmed), as: SimpleResult.self)
    }

    func getBackupConfig() async throws -> BackupConfigResponseDTO {
        try await get(path: "/backups/config", query: [:], as: BackupConfigResponseDTO.self)
    }

    func updateBackupConfig(enabled: Bool?, intervalMinutes: Int?, maxCount: Int?) async throws -> BackupConfigUpdateResultDTO {
        try await post(path: "/backups/config",
                       body: BackupConfigUpdateRequest(autoBackupEnabled: enabled,
                                                       autoBackupIntervalMinutes: intervalMinutes,
                                                       autoBackupMaxCount: maxCount),
                       as: BackupConfigUpdateResultDTO.self)
    }

    // MARK: - WebSocket

    func makeConsoleStreamTask() throws -> URLSessionWebSocketTask {
        let wsURL = try makeWebSocketURL(path: "/console/stream")
        var req = URLRequest(url: wsURL)
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        return wsSession.webSocketTask(with: req)
    }

    // MARK: - Internals

    private struct EmptyBody: Encodable { }

    private struct ActiveServerRequest: Encodable {
        let serverId: String
    }

    private struct ServerRenameRequest: Encodable {
        let serverId: String
        let name: String
    }

    private struct ServerDeleteRequest: Encodable {
        let serverId: String
    }

    private struct ServerCreateRequest: Encodable {
        let name: String?
        let serverType: String?
        let javaFlavor: String?
        let port: Int?
        let maxPlayers: Int?
        let enableCrossPlay: Bool?
        let crossPlayBedrockPort: Int?
        let enablePlayit: Bool?
        let enableXboxBroadcast: Bool?
        let difficulty: String?
        let gamemode: String?
        let worldName: String?
        let worldSeed: String?
        let versionId: String?
        let minecraftVersion: String?
        let loaderVersion: String?
        let acceptEula: Bool?
        let bedrockVersion: String?
        let dockerImage: String?
        let javaPath: String?
    }

    private struct ServerEULARequest: Encodable {
        let serverId: String
    }

    private struct ServerImportRequest: Encodable {
        let action: String
        let sourcePath: String
        let importKind: String?
        let displayName: String?
        let serverType: String?
        let activeWorldName: String?
        let port: Int?
        let maxPlayers: Int?
        let acceptEula: Bool?
        let enablePlayit: Bool?
        let transferMode: String?
        let backupPath: String?
        let javaPortOverrides: [String: Int]?
        let bedrockPortOverrides: [String: Int]?
    }

    private struct ActivateSlotRequest: Encodable {
        let slotId: String
    }

    private struct RestoreBackupRequest: Encodable {
        let backupId: String
    }

    private struct CommandRequest: Encodable {
        let command: String
    }

    private struct ComponentUpdateRequest: Encodable {
        let component: String
    }

    private struct AddonUpdateRequest: Encodable {
        let jarStem: String?
        let updateAll: Bool
    }

    private struct AddonRemoveRequest: Encodable {
        let jarStem: String
    }

    private struct CatalogInstallRequest: Encodable {
        let projectId: String
        let slug: String
        let title: String
    }

    private struct SettingsUpdateRequest: Encodable {
        let changes: [String: String]
    }

    private struct BackupConfigUpdateRequest: Encodable {
        let autoBackupEnabled: Bool?
        let autoBackupIntervalMinutes: Int?
        let autoBackupMaxCount: Int?
    }

    private struct AllowlistMutationRequest: Encodable {
        let action: String
        let name: String
    }

    private struct PlayerSkinOverrideRequest: Encodable {
        let profileId: String
        let lookupIdentifier: String?
    }

    private struct HiddenProfileMutationRequest: Encodable {
        let profileId: String
        let hidden: Bool
    }

    private struct BroadcastCredentialsRequest: Encodable {
        let email: String
        let password: String
        let gamertag: String
    }

    private func makeHTTPURL(path: String, query: [String: String]) throws -> URL {
        var comps = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)

        // Ensure path joins correctly
        let basePath = (comps?.path ?? "").trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let reqPath = path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        comps?.path = "/" + ([basePath, reqPath].filter { !$0.isEmpty }.joined(separator: "/"))

        if !query.isEmpty {
            comps?.queryItems = query.map { URLQueryItem(name: $0.key, value: $0.value) }
        }

        guard let url = comps?.url else { throw RemoteAPIError.invalidBaseURL }
        return url
    }

    private func makeWebSocketURL(path: String) throws -> URL {
        var comps = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        guard let schemeRaw = comps?.scheme?.lowercased() else { throw RemoteAPIError.invalidBaseURL }

        // Convert http/https to ws/wss; if user already entered ws/wss, keep it.
        switch schemeRaw {
        case "http":
            comps?.scheme = "ws"
        case "https":
            comps?.scheme = "wss"
        case "ws", "wss":
            break
        default:
            throw RemoteAPIError.invalidBaseURL
        }

        // Join paths like HTTP helper
        let basePath = (comps?.path ?? "").trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let reqPath = path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        comps?.path = "/" + ([basePath, reqPath].filter { !$0.isEmpty }.joined(separator: "/"))

        guard let url = comps?.url else { throw RemoteAPIError.invalidBaseURL }

        // Safety: allow WS only on local/private; allow WSS anywhere.
        if (url.scheme ?? "").lowercased() == "ws" {
            guard let host = url.host, NetworkSafety.isLocalOrPrivateHost(host) else {
                throw RemoteAPIError.insecureWebSocketNotAllowed
            }
        }

        return url
    }

    private func get<T: Decodable>(path: String, query: [String: String], as type: T.Type, urlSession: URLSession? = nil) async throws -> T {
        let url = try makeHTTPURL(path: path, query: query)

        var req = URLRequest(url: url)
        req.httpMethod = "GET"
        req.setValue("application/json", forHTTPHeaderField: "Accept")
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")

        do {
            let (data, resp) = try await (urlSession ?? session).data(for: req)
            guard let http = resp as? HTTPURLResponse else {
                throw RemoteAPIError.network("No HTTP response.")
            }

            guard (200...299).contains(http.statusCode) else {
                let body = String(data: data, encoding: .utf8)
                throw RemoteAPIError.httpStatus(http.statusCode, body)
            }

            do {
                return try JSONDecoder().decode(T.self, from: data)
            } catch {
                throw RemoteAPIError.decodingFailed
            }
        } catch let err as RemoteAPIError {
            throw err
        } catch {
            throw RemoteAPIError.network(error.localizedDescription)
        }
    }

    private func post<B: Encodable, T: Decodable>(path: String, body: B, as type: T.Type, urlSession: URLSession? = nil) async throws -> T {
        let url = try makeHTTPURL(path: path, query: [:])

        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Accept")
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")

        do {
            req.httpBody = try JSONEncoder().encode(body)
        } catch {
            throw RemoteAPIError.network("Failed to encode request.")
        }

        do {
            let (data, resp) = try await (urlSession ?? session).data(for: req)
            guard let http = resp as? HTTPURLResponse else {
                throw RemoteAPIError.network("No HTTP response.")
            }

            guard (200...299).contains(http.statusCode) else {
                let body = String(data: data, encoding: .utf8)
                throw RemoteAPIError.httpStatus(http.statusCode, body)
            }

            do {
                return try JSONDecoder().decode(T.self, from: data)
            } catch {
                throw RemoteAPIError.decodingFailed
            }
        } catch let err as RemoteAPIError {
            throw err
        } catch {
            throw RemoteAPIError.network(error.localizedDescription)
        }
    }
}
