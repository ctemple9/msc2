import Foundation
import Combine

@MainActor
final class SettingsStore: ObservableObject {
    private enum Keys {
        static let baseURL          = "msc_remote_base_url"
        static let favoriteCommands = "msc_remote_favorite_commands"
        static let recentCommands   = "msc_remote_recent_commands"
        static let showJoinCard     = "msc_remote_show_join_card"
        static let joinCardColorHex = "msc_remote_join_card_color"
        static let xboxGamertag     = "msc_remote_xbox_gamertag"
        static let accentColorHex   = "msc_remote_accent_color"
        static let allowServerUpdates = "msc_remote_allow_server_updates"
    }

    static let accentPresets: [(name: String, hex: String)] = [
        ("Jade",    "#3EB489"),
        ("Sky",     "#4A9EDB"),
        ("Violet",  "#9B6FD4"),
        ("Amber",   "#E8A838"),
        ("Coral",   "#E05C5C"),
        ("Rose",    "#D4578A"),
        ("Cyan",    "#3EC8C8"),
    ]

    enum NotificationKeys {
        static let serverWentOffline = "msc_notify_server_offline"
        static let serverCameOnline  = "msc_notify_server_online"
        static let playerJoined      = "msc_notify_player_joined"
    }

    private static let maxRecents: Int = 10

    @Published var baseURLString: String
    @Published var tokenDraft: String

    @Published private(set) var favoriteCommands: [String]
    @Published private(set) var recentCommands: [String]

    @Published var lastTestResult: String?  = nil
    @Published var lastTestWasSuccess: Bool = false

    // Notification preferences — no didSet, persisted explicitly via
    // saveNotificationPreferences(). This avoids the Swift init ordering
    // error where didSet on @Published properties prevents self access
    // before all stored properties are initialized.
    @Published var notifyServerWentOffline: Bool
    @Published var notifyServerCameOnline: Bool
    @Published var notifyPlayerJoined: Bool

    // Join card visibility, color, and Xbox gamertag, persisted to UserDefaults.
    @Published var showJoinCard: Bool
    @Published var joinCardColorHex: String
    @Published var xboxGamertag: String

    // Accent colour tint — drives MSCRemoteStyle.accent app-wide.
    @Published var accentColorHex: String

    // Safety preference for update buttons that can change server behavior.
    @Published var allowServerUpdates: Bool

    // MARK: - Init

    init() {
        self.baseURLString    = UserDefaults.standard.string(forKey: Keys.baseURL) ?? ""
        self.tokenDraft       = (try? KeychainTokenStore.loadToken()) ?? ""

        let rawFav = (UserDefaults.standard.array(forKey: Keys.favoriteCommands) as? [String]) ?? []
        let rawRec = (UserDefaults.standard.array(forKey: Keys.recentCommands) as? [String]) ?? []
        let normFav = Self.normalizeStrings(rawFav)
        let normRec = Array(Self.normalizeStrings(rawRec).prefix(Self.maxRecents))
        self.favoriteCommands = normFav
        self.recentCommands   = normRec

        // object(forKey:) as? Bool returns nil when the key was never written,
        // which lets us apply our intended defaults (true/true/false) rather
        // than UserDefaults' universal default of false.
        self.notifyServerWentOffline = UserDefaults.standard.object(forKey: NotificationKeys.serverWentOffline) as? Bool ?? true
        self.notifyServerCameOnline  = UserDefaults.standard.object(forKey: NotificationKeys.serverCameOnline)  as? Bool ?? true
        self.notifyPlayerJoined      = UserDefaults.standard.object(forKey: NotificationKeys.playerJoined)      as? Bool ?? false

        self.showJoinCard     = UserDefaults.standard.object(forKey: Keys.showJoinCard)     as? Bool   ?? false
        self.joinCardColorHex = UserDefaults.standard.string(forKey: Keys.joinCardColorHex) ?? "#2E6633"
        self.xboxGamertag     = UserDefaults.standard.string(forKey: Keys.xboxGamertag)     ?? ""
        self.allowServerUpdates = UserDefaults.standard.object(forKey: Keys.allowServerUpdates) as? Bool ?? true

        let savedAccent = UserDefaults.standard.string(forKey: Keys.accentColorHex) ?? "#3EB489"
        self.accentColorHex = savedAccent
        _mscAccentHex = savedAccent

        UserDefaults.standard.set(normFav, forKey: Keys.favoriteCommands)
        UserDefaults.standard.set(normRec, forKey: Keys.recentCommands)
    }

    func setAccentColor(_ hex: String) {
        accentColorHex = hex
        _mscAccentHex = hex
        UserDefaults.standard.set(hex, forKey: Keys.accentColorHex)
    }

    // MARK: - Notification preference persistence
    //
    // Called explicitly from SettingsView's Toggle onChange handlers.
    // No magic — just a plain write to UserDefaults.

    func saveNotificationPreferences() {
        UserDefaults.standard.set(notifyServerWentOffline, forKey: NotificationKeys.serverWentOffline)
        UserDefaults.standard.set(notifyServerCameOnline,  forKey: NotificationKeys.serverCameOnline)
        UserDefaults.standard.set(notifyPlayerJoined,      forKey: NotificationKeys.playerJoined)
    }

    func saveJoinCardPreferences() {
        UserDefaults.standard.set(showJoinCard,     forKey: Keys.showJoinCard)
        UserDefaults.standard.set(joinCardColorHex, forKey: Keys.joinCardColorHex)
        UserDefaults.standard.set(xboxGamertag,     forKey: Keys.xboxGamertag)
    }

    func saveServerUpdatePreference() {
        UserDefaults.standard.set(allowServerUpdates, forKey: Keys.allowServerUpdates)
    }

    // MARK: - Save / token management

    func save() {
        let normalizedBase = Self.normalizeHTTPBaseURLString(baseURLString)
        baseURLString = normalizedBase
        UserDefaults.standard.set(normalizedBase, forKey: Keys.baseURL)

        let trimmedToken = tokenDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedToken.isEmpty else { return }

        do {
            try KeychainTokenStore.saveToken(trimmedToken)
        } catch {
            lastTestResult     = "Save failed (Keychain)."
            lastTestWasSuccess = false
        }
    }

    func loadTokenFromKeychain() {
        tokenDraft = (try? KeychainTokenStore.loadToken()) ?? ""
    }

    func clearToken() {
        tokenDraft = ""
        try? KeychainTokenStore.deleteToken()
    }

    func resolvedBaseURL() -> URL? {
        let normalized = Self.normalizeHTTPBaseURLString(baseURLString)
        guard !normalized.isEmpty else { return nil }
        return URL(string: normalized)
    }

    func resolvedToken() -> String? {
        let t = tokenDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        return t.isEmpty ? nil : t
    }

    // MARK: - Pairing via QR / deep link

    func handleIncomingURL(_ url: URL) {
        if handlePairingURL(url) {
            lastTestResult     = "Pairing imported from link."
            lastTestWasSuccess = true
        }
    }

    func applyPairingPayload(_ payload: String) -> Bool {
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, let url = URL(string: trimmed) else { return false }
        return handlePairingURL(url)
    }

    private func handlePairingURL(_ url: URL) -> Bool {
        guard (url.scheme ?? "").lowercased() == "mscremote" else { return false }
        guard (url.host   ?? "").lowercased() == "pair"      else { return false }

        guard let comps = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return false }
        let items = comps.queryItems ?? []

        func item(_ name: String) -> String? {
            items.first(where: { $0.name.lowercased() == name.lowercased() })?.value
        }

        let base  = item("base") ?? item("baseurl") ?? item("base_url") ?? item("url")
        let token = item("token") ?? item("bearer") ?? item("t")

        applyPairing(baseURLString: base, token: token)
        return true
    }

    private func applyPairing(baseURLString: String?, token: String?) {
        if let baseURLString {
            let normalized = Self.normalizeHTTPBaseURLString(baseURLString)
            if !normalized.isEmpty {
                self.baseURLString = normalized
                UserDefaults.standard.set(normalized, forKey: Keys.baseURL)
            }
        }
        if let token {
            let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty {
                self.tokenDraft = trimmed
                do {
                    try KeychainTokenStore.saveToken(trimmed)
                } catch {
                    lastTestResult     = "Pairing import: failed to save token (Keychain)."
                    lastTestWasSuccess = false
                }
            }
        }
    }

    // MARK: - Favorites / Recents

    func isFavorite(command: String) -> Bool {
        let key = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else { return false }
        return favoriteCommands.contains(key)
    }

    func toggleFavorite(command: String) {
        let key = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else { return }
        if let idx = favoriteCommands.firstIndex(of: key) {
            favoriteCommands.remove(at: idx)
        } else {
            favoriteCommands.append(key)
        }
        favoriteCommands = Self.normalizeStrings(favoriteCommands)
        persistFavorites()
    }

    func recordRecent(command: String) {
        let key = command.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else { return }
        recentCommands.removeAll(where: { $0 == key })
        recentCommands.insert(key, at: 0)
        recentCommands = Array(Self.normalizeStrings(recentCommands).prefix(Self.maxRecents))
        persistRecents()
    }

    private func persistFavorites() {
        UserDefaults.standard.set(favoriteCommands, forKey: Keys.favoriteCommands)
    }

    private func persistRecents() {
        UserDefaults.standard.set(recentCommands, forKey: Keys.recentCommands)
    }

    // MARK: - Static helpers

    private static func normalizeStrings(_ values: [String]) -> [String] {
        var seen = Set<String>()
        var out: [String] = []
        for raw in values {
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            if seen.insert(trimmed).inserted { out.append(trimmed) }
        }
        return out
    }

    private static func normalizeHTTPBaseURLString(_ raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "" }
        let lower = trimmed.lowercased()
        if lower.hasPrefix("http://") || lower.hasPrefix("https://") { return trimmed }
        if trimmed.contains("://") { return trimmed }
        return "http://\(trimmed)"
    }
}


