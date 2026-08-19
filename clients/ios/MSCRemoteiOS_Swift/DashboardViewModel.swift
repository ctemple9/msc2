import Foundation
import Combine

@MainActor
final class DashboardViewModel: ObservableObject {
    @Published var status: RemoteAPIStatus? = nil
    @Published var servers: [ServerDTO] = []
    @Published var consoleTail: [ConsoleLineDTO] = []
    @Published var consoleStream: [ConsoleLineDTO] = []
    @Published var players: PlayersResponse? = nil

    struct PerformancePoint: Identifiable, Equatable {
        let id = UUID()
        let timestamp: Date
        let tps1m: Double?
        let playersOnline: Int?
        let cpuPercent: Double?
        let ramUsedMB: Double?
        let ramMaxMB: Double?
        let worldSizeMB: Double?
    }

    @Published var performanceLatest: PerformancePoint? = nil
    @Published var performanceHistory: [PerformancePoint] = []
    @Published var performanceErrorMessage: String? = nil

    @Published var isLoading: Bool = false
    @Published var errorMessage: String? = nil
    @Published var lastUpdated: Date? = nil
    @Published var isStreamingConsole: Bool = false

    @Published var componentsStatus: ComponentsStatusDTO? = nil
    @Published var broadcastStatus: BroadcastStatusDTO? = nil
    @Published var pendingAuthPrompt: BroadcastAuthPromptDTO? = nil
    @Published var broadcastAutoStart: Bool? = nil

    /// "admin", "guest", or "named" — nil = not yet determined or not connected.
    @Published var connectedRole: String? = nil
    /// Display name for named tokens; nil for legacy admin/guest tokens.
    @Published var connectedName: String? = nil
    /// Permission categories granted to this token; nil = no restriction (admin).
    @Published var connectedPermissions: [String]? = nil

    @Published var usersResponse: UserListResponseDTO? = nil

    @Published var worldsResponse: WorldSlotsResponseDTO? = nil
    @Published var backupsResponse: BackupsResponseDTO? = nil
    /// Latest snapshot of a long-running world/backup operation this view
    /// model started (`convertWorld`), published on every poll so a P6.24
    /// UI can show live progress. `nil` when nothing is in flight.
    @Published var activeOperation: OperationDTO? = nil
    @Published var allowlistResponse: AllowlistResponseDTO? = nil
    @Published var sessionLogResponse: SessionLogResponseDTO? = nil
    @Published var playerProfilesResponse: PlayerProfilesResponseDTO? = nil
    @Published var playerSkinResponses: [String: PlayerSkinResponseDTO] = [:]
    @Published var addonsResponse: AddonsResponseDTO? = nil
    @Published var settingsResponse: SettingsResponseDTO? = nil
    @Published var backupConfigResponse: BackupConfigResponseDTO? = nil
    @Published var healthResponse: HealthResponseDTO? = nil
    @Published var healthProblemsResponse: HealthProblemsResponseDTO? = nil
    @Published var isLoadingHealth: Bool = false
    @Published var connectivityResponse: ConnectivityResponseDTO? = nil
    @Published var playitStatusResponse: PlayitStatusResponseDTO? = nil
    @Published var duckdnsResponse: DuckDNSStatusResponseDTO? = nil
    @Published var geyserConfigResponse: GeyserConfigResponseDTO? = nil
    @Published var ramConfigResponse: RAMConfigResponseDTO? = nil
    @Published var serverImportScanResponse: ServerImportScanResponseDTO? = nil
    @Published var serverFilesResponse: ServerFilesResponseDTO? = nil
    @Published var serverFileReadResponse: ServerFileReadResponseDTO? = nil
    @Published var clientExportResponse: ClientExportResponseDTO? = nil

    var notifications: NotificationManager = .shared

    // MARK: - Internal state (do not access from views)
    // These properties are managed exclusively by the DashboardViewModel extension
    // files (ConsoleStream, Performance, Notifications). Do not read or write them
    // from SwiftUI views or other callsites outside this class and its extensions.
    var previousRunning: Bool? = nil
    var previousPlayerNames: Set<String> = []

    let maxPerformanceSamples: Int = 60
    var client: RemoteAPIClient? = nil
    var clientBaseURL: URL? = nil
    var clientToken: String? = nil
    var webSocketTask: URLSessionWebSocketTask? = nil
    var webSocketReceiveTask: Task<Void, Never>? = nil

    func updateCredentials(baseURL: URL, token: String) {
        guard baseURL != clientBaseURL || token != clientToken else { return }
        clientBaseURL = baseURL
        clientToken = token
        client = nil
        connectedRole = nil
        connectedName = nil
        connectedPermissions = nil
    }

    func requireClient() throws -> RemoteAPIClient {
        if let existing = client { return existing }
        guard let baseURL = clientBaseURL, let token = clientToken else {
            throw RemoteAPIError.missingToken
        }
        let newClient = try RemoteAPIClient(baseURL: baseURL, token: token)
        client = newClient
        return newClient
    }

    func refreshAll(baseURL: URL, token: String, tailN: Int) async {
        updateCredentials(baseURL: baseURL, token: token)

        isLoading = true
        errorMessage = nil

        let runningBeforeRefresh = previousRunning
        let playerNamesBeforeRefresh = previousPlayerNames

        do {
            let client = try requireClient()

            // Status is fetched on its own: against the Phase 2 skeletal agent, `getServers()`
            // and `getConsoleTail()` 404 (those routes don't exist yet), and bundling all three
            // into one `try await` would let that sibling failure blank out a status fetch that
            // actually succeeded. Non-fatal here — a missing server list / console tail just
            // leaves the previous values in place.
            async let s2 = try? client.getServers()
            async let s3 = try? client.getConsoleTail(n: tailN)

            let fetchedStatus = try await client.getStatus()
            status = fetchedStatus
            lastUpdated = Date()

            if let fetchedServers = await s2 { servers = fetchedServers }
            if let fetchedTail = await s3 { consoleTail = fetchedTail }
        } catch {
            errorMessage = error.localizedDescription
        }

        await fetchPerformanceSnapshot(baseURL: baseURL, token: token)
        await fetchComponentsAndBroadcast(baseURL: baseURL, token: token)

        if connectedRole == nil {
            if let c = try? requireClient(), let me = try? await c.getMe() {
                connectedRole = me.role
                connectedName = me.name
                connectedPermissions = me.permissions
            }
        }

        isLoading = false

        await fetchPlayers(baseURL: baseURL, token: token)

        evaluateNotifications(
            previousRunning: runningBeforeRefresh,
            previousPlayerNames: playerNamesBeforeRefresh
        )
    }

    func pollStatusAndPlayers(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)

        let runningBeforeRefresh = previousRunning
        let playerNamesBeforeRefresh = previousPlayerNames

        do {
            let client = try requireClient()

            async let s1 = client.getStatus()
            async let s2 = client.getPlayers()
            async let s3auth = try? client.getAuthPrompt()
            async let s3broad = try? client.getBroadcastStatus()

            if servers.isEmpty {
                async let s3 = client.getServers()
                let (fetchedStatus, fetchedPlayers, fetchedServers, auth, broad) = try await (s1, s2, s3, s3auth, s3broad)
                status = fetchedStatus
                players = fetchedPlayers
                servers = fetchedServers
                broadcastStatus = broad
                if let auth, auth.isPresent { pendingAuthPrompt = auth } else { pendingAuthPrompt = nil }
            } else {
                let (fetchedStatus, fetchedPlayers, auth, broad) = try await (s1, s2, s3auth, s3broad)
                status = fetchedStatus
                players = fetchedPlayers
                broadcastStatus = broad
                if let auth, auth.isPresent { pendingAuthPrompt = auth } else { pendingAuthPrompt = nil }
            }

            lastUpdated = Date()

            evaluateNotifications(
                previousRunning: runningBeforeRefresh,
                previousPlayerNames: playerNamesBeforeRefresh
            )
        } catch {
        }
    }

    func setActiveServer(baseURL: URL, token: String, serverId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().setActiveServer(serverId: serverId)
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func renameServer(baseURL: URL, token: String, serverId: String, name: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().renameServer(serverId: serverId, name: name)
            guard result.success else { return friendlyServerManagementError(result.message) }
            let newName = result.name ?? name.trimmingCharacters(in: .whitespacesAndNewlines)
            servers = servers.map { server in
                guard server.id == serverId else { return server }
                return ServerDTO(id: server.id, name: newName, directory: server.directory,
                                 serverType: server.serverType, javaFlavor: server.javaFlavor,
                                 gamePort: server.gamePort,
                                 hostAddress: server.hostAddress)
            }
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func deleteServer(baseURL: URL, token: String, serverId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().deleteServer(serverId: serverId)
            guard result.success else { return friendlyServerManagementError(result.message) }
            servers.removeAll { $0.id == serverId }
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func createFreshServer(baseURL: URL, token: String, name: String, serverType: ServerType,
                           javaFlavor: RemoteJavaServerFlavor?, selectedVersion: VersionEntryDTO?, port: Int,
                           maxPlayers: Int?, enableCrossPlay: Bool, crossPlayBedrockPort: Int,
                           enablePlayit: Bool, enableXboxBroadcast: Bool, difficulty: String, gamemode: String,
                           worldName: String?, worldSeed: String?, acceptEula: Bool,
                           bedrockVersion: String?, dockerImage: String?,
                           javaPath: String? = nil) async -> (error: String?, warnings: [String]?) {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().createFreshServer(
                name: name,
                serverType: serverType,
                javaFlavor: javaFlavor,
                selectedVersion: selectedVersion,
                port: port,
                maxPlayers: maxPlayers,
                enableCrossPlay: enableCrossPlay,
                crossPlayBedrockPort: crossPlayBedrockPort,
                enablePlayit: enablePlayit,
                enableXboxBroadcast: enableXboxBroadcast,
                difficulty: difficulty,
                gamemode: gamemode,
                worldName: worldName,
                worldSeed: worldSeed,
                acceptEula: acceptEula,
                bedrockVersion: bedrockVersion,
                dockerImage: dockerImage,
                javaPath: javaPath
            )
            guard result.success else { return (friendlyServerManagementError(result.message), nil) }
            // P7.9's own correction: this 200 only means "creation admitted
            // and started," not "the server now exists" — poll the real
            // operation to its terminal state before treating this as
            // done, the same way `activateWorld`/`convertWorld` already do
            // for their own long-running work.
            if let operationId = result.operationId {
                do {
                    let operation = try await requireClient().pollOperationToTerminal(id: operationId)
                    switch operation.state {
                    case .failed:
                        let message = operation.error?.message ?? "Server creation failed."
                        return (message, nil)
                    case .cancelled:
                        return ("Server creation was cancelled.", nil)
                    case .succeeded, .queued, .running:
                        break
                    }
                } catch {
                    // Inconclusive -- the poll itself failed (e.g. the app
                    // lost connectivity mid-install), not the creation.
                    // Fall through and refresh; the agent's own operation
                    // record is the durable source of truth either way.
                }
            }
            if let fetched = try? await requireClient().getServers() { servers = fetched }
            if let fetchedStatus = try? await requireClient().getStatus() { status = fetchedStatus }
            return (nil, result.warnings)
        } catch {
            errorMessage = error.localizedDescription
            return (error.localizedDescription, nil)
        }
    }

    func fetchBroadcastJarStatus(baseURL: URL, token: String) async -> BroadcastJarStatusDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getBroadcastJarStatus()
    }

    func triggerBroadcastJarDownload(baseURL: URL, token: String) async -> BroadcastJarDownloadResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.downloadBroadcastJar()
    }

    func fetchJavaRuntimes(baseURL: URL, token: String) async -> JavaRuntimesResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getJavaRuntimes()
    }

    func fetchJavaConfig(baseURL: URL, token: String) async -> JavaConfigResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getJavaConfig()
    }

    func applyJavaConfig(baseURL: URL, token: String, executablePath: String?) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return false }
        do {
            try await client.setJavaConfig(executablePath: executablePath)
            return true
        } catch {
            return false
        }
    }

    func fetchCreateVersions(baseURL: URL, token: String, serverType: ServerType,
                             javaFlavor: RemoteJavaServerFlavor?) async -> VersionsResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getCreateVersions(serverType: serverType, javaFlavor: javaFlavor)
    }

    func acceptServerEULA(baseURL: URL, token: String, serverId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().acceptServerEULA(serverId: serverId)
            guard result.success else { return friendlyServerManagementError(result.message) }
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func scanServerImport(baseURL: URL, token: String, sourcePath: String, importKind: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().scanServerImport(sourcePath: sourcePath, importKind: importKind)
            guard result.success else { return friendlyServerManagementError(result.message) }
            serverImportScanResponse = result
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func importExistingServer(baseURL: URL, token: String, sourcePath: String, importKind: String,
                              displayName: String, serverType: ServerType, activeWorldName: String?,
                              port: Int?, maxPlayers: Int?, acceptEula: Bool,
                              enablePlayit: Bool) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        activeOperation = nil
        do {
            let client = try requireClient()
            let result = try await client.importExistingServer(
                sourcePath: sourcePath,
                importKind: importKind,
                displayName: displayName,
                serverType: serverType,
                activeWorldName: activeWorldName,
                port: port,
                maxPlayers: maxPlayers,
                acceptEula: acceptEula,
                enablePlayit: enablePlayit
            )
            return await finishServerImport(result, client: client)
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func importTransferPackage(baseURL: URL, token: String, sourcePath: String, replaceAll: Bool, backupPath: String?) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        activeOperation = nil
        do {
            let client = try requireClient()
            let result = try await client.importTransferPackage(sourcePath: sourcePath, replaceAll: replaceAll, backupPath: backupPath)
            return await finishServerImport(result, client: client)
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    private func finishServerImport(_ receipt: ServerImportResultDTO, client: RemoteAPIClient) async -> String? {
        guard receipt.success else { return friendlyServerManagementError(receipt.message) }

        do {
            if let terminal = try await client.pollServerImportToTerminal(receipt) { [weak self] update in
                self?.activeOperation = update
            } {
                switch terminal.state {
                case .succeeded:
                    break
                case .failed:
                    let message = terminal.error?.message ?? terminal.statusLine ?? "Server import failed."
                    errorMessage = message
                    return message
                case .cancelled:
                    let message = terminal.error?.message ?? terminal.statusLine ?? "Server import was cancelled."
                    errorMessage = message
                    return message
                case .queued, .running:
                    preconditionFailure("operation poll returned before reaching a terminal state")
                }
            }

            if let fetched = try? await client.getServers() { servers = fetched }
            if let fetchedStatus = try? await client.getStatus() { status = fetchedStatus }
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    private func friendlyServerManagementError(_ message: String) -> String {
        switch message {
        case "name_required": return "Enter a server name."
        case "invalid_server_type": return "Choose Java or Bedrock."
        case "invalid_java_flavor": return "Choose a supported Java server type."
        case "create_failed": return "The Mac could not create the server. Check the Mac console."
        case "missing_source_path": return "Enter a path on the Mac."
        case "source_not_found": return "The Mac could not find that path."
        case "backup_path_required": return "Enter a backup path before replacing all servers."
        case "display_name_required": return "Enter a server name."
        case "server_not_found": return "That server no longer exists."
        case "server_running": return "Stop the server before deleting it."
        case "delete_failed": return "The Mac could not delete that server folder."
        case "unsupported_server_type": return "EULA acceptance is only needed for Java servers."
        case "eula_write_failed": return "The Mac could not write eula.txt."
        case "not_available": return "This Mac has not enabled server management yet."
        default: return message
        }
    }

    func start(baseURL: URL, token: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().start()
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func stop(baseURL: URL, token: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().stop()
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func restart(baseURL: URL, token: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().restart()
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func sendCommand(baseURL: URL, token: String, command: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().sendCommand(command)
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    @MainActor func fetchComponentsAndBroadcast(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return }
        async let c = try? client.getComponents()
        async let b = try? client.getBroadcastStatus()
        async let a = try? client.getAuthPrompt()
        async let s = try? client.getBroadcastAutoStart()
        let (comp, broad, auth, autoStart) = await (c, b, a, s)
        componentsStatus = comp
        broadcastStatus = broad
        broadcastAutoStart = autoStart?.enabled
        if let auth, auth.isPresent { pendingAuthPrompt = auth } else { pendingAuthPrompt = nil }
    }

    func dismissAuthPrompt(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        _ = try? await requireClient().dismissAuthPrompt()
        pendingAuthPrompt = nil
    }

    // MARK: - Session Log & Player Profiles

    func fetchSessionLog(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            sessionLogResponse = try await requireClient().getSessionLog()
        } catch {
            // non-fatal — session log is supplementary
        }
    }

    func fetchPlayerProfiles(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let response = try await requireClient().getPlayerProfiles()
            playerProfilesResponse = response
            // If the server just triggered NBT loading for some profiles, retry once
            // after a short delay so stats are available on the next render.
            if response.isLoadingStats {
                try? await Task.sleep(nanoseconds: 2_500_000_000)
                if let updated = try? await requireClient().getPlayerProfiles() {
                    playerProfilesResponse = updated
                }
            }
        } catch {
            // non-fatal
        }
    }

    func fetchPlayerSkin(baseURL: URL, token: String, profileId: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        guard playerSkinResponses[profileId]?.success != true else { return }
        do {
            let response = try await requireClient().getPlayerSkin(profileId: profileId)
            playerSkinResponses[profileId] = response
        } catch {
            // non-fatal; rows fall back to generic avatar.
        }
    }

    func setPlayerSkinOverride(baseURL: URL, token: String, profileId: String, lookupIdentifier: String?) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().setPlayerSkinOverride(profileId: profileId, lookupIdentifier: lookupIdentifier)
            guard result.success else { return friendlyPlayerError(result.message) }
            playerSkinResponses.removeValue(forKey: profileId)
            await fetchPlayerProfiles(baseURL: baseURL, token: token)
            await fetchPlayerSkin(baseURL: baseURL, token: token, profileId: profileId)
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func setHiddenProfile(baseURL: URL, token: String, profileId: String, hidden: Bool) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().setHiddenProfile(profileId: profileId, hidden: hidden)
            guard result.success else { return friendlyPlayerError(result.message) }
            await fetchPlayerProfiles(baseURL: baseURL, token: token)
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    private func friendlyPlayerError(_ message: String) -> String {
        switch message {
        case "missing_profile_id": return "Missing player profile."
        case "profile_not_found": return "That player profile is no longer available."
        case "no_active_server": return "No active server is selected on the Mac."
        case "not_available": return "This Mac has not enabled player profile management yet."
        default: return message
        }
    }

    // MARK: - Server Files & Client Export

    func fetchServerFiles(baseURL: URL, token: String, path: String?) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            serverFilesResponse = try await requireClient().getServerFiles(path: path)
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func readServerFile(baseURL: URL, token: String, path: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().readServerFile(path: path)
            guard result.success else { return friendlyFileError(result.message) }
            serverFileReadResponse = result
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    func fetchClientExport(baseURL: URL, token: String, selectedIds: [String]? = nil) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            clientExportResponse = try await requireClient().getClientExport(selectedIds: selectedIds)
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    private func friendlyFileError(_ message: String) -> String {
        switch message {
        case "missing_path": return "Choose a file first."
        case "file_not_found": return "That file is no longer available."
        case "directory_not_file": return "Choose a file, not a folder."
        case "not_previewable": return "That file type cannot be previewed."
        case "read_failed": return "That file couldn't be read on the Mac."
        case "no_active_server": return "No active server is selected on the Mac."
        default: return message
        }
    }

    // MARK: - Worlds & Backups

    func fetchWorlds(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            worldsResponse = try await requireClient().getWorlds()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func fetchBackups(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            backupsResponse = try await requireClient().getBackups()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func fetchBackupConfig(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        backupConfigResponse = try? await (try requireClient()).getBackupConfig()
    }

    /// Returns nil on success, an error string on failure. On success, backupConfigResponse is updated from the echoed config.
    func updateBackupConfig(baseURL: URL, token: String, enabled: Bool? = nil, intervalMinutes: Int? = nil, maxCount: Int? = nil) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateBackupConfig(enabled: enabled, intervalMinutes: intervalMinutes, maxCount: maxCount)
            if result.success, let fresh = result.config {
                backupConfigResponse = fresh
                return nil
            }
            return result.message
        } catch {
            return error.localizedDescription
        }
    }

    func activateWorldSlot(baseURL: URL, token: String, slotId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().activateWorldSlot(slotId: slotId)
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    // MARK: - World management verbs (P9)
    // Each returns the error string on failure, nil on success. On success the echoed
    // slot list is applied to worldsResponse immediately.

    func createWorld(baseURL: URL, token: String, name: String, seed: String?) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().createWorld(name: name, seed: seed)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    func renameWorld(baseURL: URL, token: String, slotId: String, name: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().renameWorld(slotId: slotId, name: name)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    func replaceWorld(baseURL: URL, token: String, slotId: String, sourceSlotId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().replaceWorld(slotId: slotId, sourceSlotId: sourceSlotId)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    func repairWorld(baseURL: URL, token: String, slotId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().repairWorld(slotId: slotId)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    // MARK: - World management verbs: Phase 6 additions
    // Same shape as the P9 verbs above: nil on success (with worldsResponse
    // refreshed from the echoed slot list), an error string on failure.

    func deleteWorldSlot(baseURL: URL, token: String, slotId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().deleteWorldSlot(slotId: slotId)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    func duplicateWorldSlot(baseURL: URL, token: String, slotId: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().duplicateWorldSlot(slotId: slotId)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    /// Renames the active/live world's on-disk folders directly --
    /// distinct from `renameWorld(slotId:name:)` above, which only
    /// renames a saved slot's metadata.
    func renameActiveWorld(baseURL: URL, token: String, name: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().renameActiveWorld(name: name)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    /// Uploads a local world ZIP's bytes as a new saved slot.
    func importWorldZip(baseURL: URL, token: String, name: String, data: Data) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().importWorldZip(name: name, data: data)
            guard result.success else { return worldErrorText(result.message) }
            if let fresh = result.updated { worldsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    /// Downloads a saved slot's archive bytes. Returns nil on failure
    /// (`errorMessage` is set); the caller decides where to write them.
    func exportWorldSlot(baseURL: URL, token: String, slotId: String) async -> Data? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            return try await requireClient().exportWorldSlot(slotId: slotId)
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    /// Starts a world conversion and polls it to a terminal state,
    /// publishing each update to `activeOperation`. Returns the terminal
    /// `OperationDTO` (inspect its `.state`/`.error`) or nil on a
    /// transport failure (`errorMessage` is set). Unlike the P9 verbs
    /// above, conversion is always operation-backed -- there is no
    /// synchronous result to fall back on.
    func convertWorld(baseURL: URL, token: String, sourceSlotId: String, targetServerId: String,
                      targetFormat: String, targetName: String?, targetSlotId: String?) async -> OperationDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        activeOperation = nil
        do {
            let client = try requireClient()
            let started = try await client.convertWorld(sourceSlotId: sourceSlotId, targetServerId: targetServerId,
                                                         targetFormat: targetFormat, targetName: targetName,
                                                         targetSlotId: targetSlotId)
            return try await client.pollOperationToTerminal(id: started.operationId) { [weak self] update in
                self?.activeOperation = update
            }
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    func deleteBackup(baseURL: URL, token: String, backupId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().deleteBackup(backupId: backupId)
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    /// Requests server-side cancellation of a still-in-flight operation
    /// (`world convert`'s own Cancel button). Real at the operation-
    /// record level only -- the same caveat `RemoteAPIClient
    /// .pollOperationToTerminal`'s own doc names: the underlying
    /// filesystem/process work may still run to completion in the
    /// background. Errors are swallowed to `false` rather than surfaced
    /// through `errorMessage`, since a UI mid-cancel cares only about
    /// whether the request landed, not about replacing whatever
    /// in-progress status it's already showing.
    func cancelOperation(baseURL: URL, token: String, operationId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            _ = try await requireClient().cancelOperation(id: operationId)
            return true
        } catch {
            return false
        }
    }

    /// Maps a server world-mutation error code to a friendly message.
    private func worldErrorText(_ code: String) -> String {
        switch code {
        case "no_active_server":   return "No active server."
        case "name_required":      return "Enter a world name."
        case "slot_not_found":     return "That world slot no longer exists."
        case "source_not_found":   return "The source world could not be found."
        case "same_slot":          return "Pick a different world to copy from."
        case "server_running":     return "Stop the server before repairing."
        case "bedrock_only":       return "Repair is only available for Bedrock servers."
        case "not_active_slot":    return "Repair only works on the active world."
        case "repair_in_progress": return "A repair is already running."
        case "create_failed":      return "Could not create the world. Check the Mac console."
        case "replace_failed":     return "Could not replace the world. Check the Mac console."
        default:                   return code
        }
    }

    // MARK: - Connectivity (P11)

    func fetchConnectivity(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        connectivityResponse = try? await (try requireClient()).getConnectivity()
    }

    // MARK: - Playit tunnel (P12)

    func fetchPlayitStatus(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        playitStatusResponse = try? await (try requireClient()).getPlayitStatus()
    }

    /// Returns "started" | "already_running" on success, or nil on network error.
    func startPlayit(baseURL: URL, token: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            return try await requireClient().startPlayit().result
        } catch { return nil }
    }

    /// Returns "stopped" | "not_running" on success, or nil on network error.
    func stopPlayit(baseURL: URL, token: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            return try await requireClient().stopPlayit().result
        } catch { return nil }
    }

    // MARK: - DuckDNS (P13)

    func fetchDuckDNS(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        duckdnsResponse = try? await (try requireClient()).getDuckDNSStatus()
    }

    /// Returns true on success. Admin-only endpoint; will throw/return false for guests.
    func updateDuckDNS(hostname: String?, baseURL: URL, token: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateDuckDNS(hostname: hostname)
            if result.success { duckdnsResponse = DuckDNSStatusResponseDTO(hostname: result.hostname, isConfigured: result.hostname != nil && !(result.hostname?.isEmpty ?? true)) }
            return result.success
        } catch { return false }
    }

    // MARK: - Geyser config (P13)

    func fetchGeyserConfig(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        geyserConfigResponse = try? await (try requireClient()).getGeyserConfig()
    }

    func updateGeyserConfig(address: String?, port: Int?, baseURL: URL, token: String) async -> GeyserConfigUpdateResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateGeyserConfig(address: address, port: port)
            if result.success, let prev = geyserConfigResponse {
                geyserConfigResponse = GeyserConfigResponseDTO(
                    serverName: prev.serverName, serverType: prev.serverType,
                    isGeyserInstalled: prev.isGeyserInstalled,
                    address: result.address, port: result.port,
                    configFileExists: prev.configFileExists, note: prev.note
                )
            }
            return result
        } catch { return nil }
    }

    // MARK: - RAM allocation (/config/ram)

    func fetchRAMConfig(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        ramConfigResponse = try? await (try requireClient()).getRAMConfig()
    }

    /// Saves a new RAM allocation. Admin-only endpoint; returns nil on failure.
    /// On success the local `ramConfigResponse` is patched with the clamped values.
    func updateRAMConfig(minRamGB: Double?, maxRamGB: Double?, baseURL: URL, token: String) async -> RAMConfigUpdateResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateRAMConfig(minRamGB: minRamGB, maxRamGB: maxRamGB)
            if result.success, let prev = ramConfigResponse {
                ramConfigResponse = RAMConfigResponseDTO(
                    serverName: prev.serverName,
                    serverType: prev.serverType,
                    minRamGB: result.minRamGB ?? prev.minRamGB,
                    maxRamGB: result.maxRamGB ?? prev.maxRamGB,
                    physicalRAMGB: prev.physicalRAMGB,
                    recommendedMaxGB: prev.recommendedMaxGB,
                    serverRunning: prev.serverRunning,
                    hasActiveServer: prev.hasActiveServer
                )
            }
            return result
        } catch { return nil }
    }

    // MARK: - Diagnostics (health + startup problems)

    func fetchHealth(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            healthResponse = try await requireClient().getHealth()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func fetchHealthProblems(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        healthProblemsResponse = try? await (try requireClient()).getHealthProblems()
    }

    /// Triggers a repair. Returns nil on success (applying the echoed problems list),
    /// or a friendly error string on failure.
    func repairHealthProblem(baseURL: URL, token: String, problemId: String, action: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().repairHealthProblem(problemId: problemId, action: action)
            guard result.success else { return healthRepairErrorText(result.message) }
            if let fresh = result.updated { healthProblemsResponse = fresh }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    private func healthRepairErrorText(_ code: String) -> String {
        switch code {
        case "server_running":     return "Stop the server before repairing mods."
        case "no_active_server":   return "No active server."
        case "problem_not_found":  return "That problem is no longer listed."
        case "action_unavailable": return "That fix isn't available for this problem."
        case "invalid_action":     return "Unknown repair action."
        default:                   return code
        }
    }

    // MARK: - Allowlist (Bedrock)

    func fetchAllowlist(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            allowlistResponse = try await requireClient().getAllowlist()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// Adds or removes an allowlist entry. Returns the error message on failure,
    /// nil on success. On success the returned list is applied immediately.
    func mutateAllowlist(baseURL: URL, token: String, action: String, name: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            let result = try await requireClient().mutateAllowlist(action: action, name: name)
            guard result.success else { return result.message }
            allowlistResponse = AllowlistResponseDTO(serverType: result.serverType, entries: result.entries)
            return nil
        } catch {
            errorMessage = error.localizedDescription
            return error.localizedDescription
        }
    }

    // MARK: - Settings

    func fetchSettings(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return }
        settingsResponse = try? await client.getSettings()
    }

    /// Applies a sparse change set. Returns the result (nil on transport error).
    /// On success the echoed fresh schema is applied to `settingsResponse`.
    func updateSettings(baseURL: URL, token: String, changes: [String: String]) async -> SettingsUpdateResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateSettings(changes: changes)
            if let current = settingsResponse, let fresh = result.sections {
                settingsResponse = SettingsResponseDTO(
                    serverType: current.serverType,
                    serverName: current.serverName,
                    serverRunning: current.serverRunning,
                    editable: current.editable,
                    sections: fresh,
                    note: current.note
                )
            }
            return result
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    // MARK: - Add-ons

    func fetchAddons(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return }
        addonsResponse = try? await client.getAddons()
    }

    /// Returns the result string on fire-and-forget success ("update_started"), or an error description.
    func updateAddon(baseURL: URL, token: String, jarStem: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateAddon(jarStem: jarStem)
            return result.result
        } catch {
            return error.localizedDescription
        }
    }

    func updateAllAddons(baseURL: URL, token: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().updateAllAddons()
            return result.result
        } catch {
            return error.localizedDescription
        }
    }

    /// Returns nil on success (also optimistically removes the addon from addonsResponse),
    /// or an error/message string on failure.
    func removeAddon(baseURL: URL, token: String, jarStem: String) async -> String? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().removeAddon(jarStem: jarStem)
            guard result.success else { return result.message }
            if let current = addonsResponse {
                addonsResponse = AddonsResponseDTO(
                    addons: current.addons.filter { $0.jarStem != jarStem },
                    isResolving: current.isResolving,
                    serverSupportsAddons: current.serverSupportsAddons,
                    packManaged: current.packManaged,
                    packName: current.packName
                )
            }
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    // MARK: - Catalog (search + install)

    /// Searches the add-on catalog. Returns nil on network error (caller shows a message).
    func searchCatalog(baseURL: URL, token: String, query: String, offset: Int = 0) async -> CatalogSearchResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.searchCatalog(query: query, offset: offset)
    }

    /// Installs a catalog add-on. Returns the result on completion, or nil if the request
    /// itself threw (timeout/network) — the caller treats nil as "inconclusive" and refreshes.
    func installCatalogAddon(baseURL: URL, token: String, item: CatalogItemDTO) async -> CatalogInstallResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.installAddon(projectId: item.projectId, slug: item.slug, title: item.title)
    }

    // MARK: - Resource Packs

    func fetchResourcePacks(baseURL: URL, token: String) async -> ResourcePacksResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getResourcePacks()
    }

    func activateResourcePack(baseURL: URL, token: String, packId: String?, require: Bool = false) async -> ResourcePackMutationResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.activateResourcePack(packId: packId, require: require)
    }

    func setResourcePackURL(baseURL: URL, token: String, url: String, sha1: String?, require: Bool) async -> ResourcePackMutationResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.setResourcePackURL(url: url, sha1: sha1, require: require)
    }

    func toggleGeyserPack(baseURL: URL, token: String, packId: String, enabled: Bool) async -> ResourcePackMutationResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.toggleGeyserPack(packId: packId, enabled: enabled)
    }

    func removeResourcePack(baseURL: URL, token: String, packId: String, packKind: String) async -> ResourcePackMutationResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.removeResourcePack(packId: packId, packKind: packKind)
    }

    // MARK: - Versions (server JAR picker)

    /// Fetches available versions for the active server's flavor.
    func fetchVersions(baseURL: URL, token: String) async -> VersionsResponseDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return nil }
        return try? await client.getVersions()
    }

    /// Downloads / installs the given version, polling the real operation
    /// to a terminal state before reporting success. `server_running`/
    /// `download_in_progress`/`no_active_server`/`not_supported` never
    /// reach `.succeeded` or `.failed` here — the frozen contract refuses
    /// those synchronously (a thrown `RemoteAPIError.httpStatus`, not a
    /// `VersionChangeResultDTO`), so this method surfaces them as
    /// `.refused(code:message:)` instead of the P7.26-found-stale
    /// "check `result.message`" pattern this replaces.
    enum VersionChangeOutcome {
        case succeeded(VersionChangeResultDTO)
        case refused(code: String?, message: String)
        /// The request itself, or the follow-up operation poll, failed
        /// inconclusively (e.g. the app lost connectivity) — the change
        /// may still be running or have completed on the agent.
        case inconclusive
    }

    func changeVersion(baseURL: URL, token: String, entry: VersionEntryDTO) async -> VersionChangeOutcome {
        updateCredentials(baseURL: baseURL, token: token)
        guard let client = try? requireClient() else { return .inconclusive }
        do {
            let result = try await client.changeVersion(versionId: entry.id, loaderVersion: entry.loaderVersion)
            guard let operationId = result.operationId else { return .succeeded(result) }
            do {
                let operation = try await client.pollOperationToTerminal(id: operationId)
                switch operation.state {
                case .succeeded:
                    return .succeeded(result)
                case .failed:
                    return .refused(code: operation.error?.code, message: operation.error?.message ?? "Version change failed.")
                case .cancelled:
                    return .refused(code: nil, message: "Version change was cancelled.")
                case .queued, .running:
                    return .inconclusive
                }
            } catch {
                return .inconclusive
            }
        } catch let err as RemoteAPIError {
            return .refused(code: err.apiErrorCode, message: err.localizedDescription)
        } catch {
            return .inconclusive
        }
    }

    func createBackupNow(baseURL: URL, token: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().createBackupNow()
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func restoreBackup(baseURL: URL, token: String, backupId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        do {
            _ = try await requireClient().restoreBackup(backupId: backupId)
            return true
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    // MARK: - Permission helpers

    /// True if this token is an admin or holds the named permission category.
    func hasPermission(_ permission: String) -> Bool {
        guard let role = connectedRole else { return false }
        if role == "admin" { return true }
        if role == "guest" { return false }
        return connectedPermissions?.contains(permission) ?? false
    }

    // MARK: - User management (P17)

    func fetchUsers(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            usersResponse = try await requireClient().getUsers()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func createUser(baseURL: URL, token: String, label: String, role: String, permissions: [String]?, expiresInDays: Int?) async -> UserCreateResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            return try await requireClient().createUser(label: label, role: role, permissions: permissions, expiresInDays: expiresInDays)
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    func revokeUser(baseURL: URL, token: String, userId: String) async -> Bool {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().revokeUser(userId: userId)
            return result.success
        } catch {
            errorMessage = error.localizedDescription
            return false
        }
    }

    func updateUser(baseURL: URL, token: String, userId: String, label: String?, role: String?, permissions: [String]?, expiresInDays: Int?) async -> UserUpdateResultDTO? {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            return try await requireClient().updateUser(userId: userId, label: label, role: role, permissions: permissions, expiresInDays: expiresInDays)
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }
}
